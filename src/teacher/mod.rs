use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{TeacherBackendKind, TeacherConfig};
use crate::curriculum::{
    CurriculumBundle, CurriculumOutline, CurriculumRequest, build_curriculum_from_outline,
    build_mock_curriculum, outline_catalog_reference,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeacherCallRecord {
    pub teacher_call_id: String,
    pub session_id: String,
    pub timestamp: String,
    pub teacher_backend_id: String,
    pub operation: String,
    pub request_payload: serde_json::Value,
    pub response_payload: serde_json::Value,
    pub token_counts: Option<TokenCounts>,
    pub latency_ms: u64,
    pub model_config: serde_json::Value,
    pub runtime_config: serde_json::Value,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenCounts {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeacherEvaluation {
    pub correctness_judgment: String,
    pub feedback: String,
    pub caution_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeacherAdvice {
    pub next_step: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeacherSessionNotes {
    pub summary: String,
    pub caution_notes: Vec<String>,
}

pub trait TeacherBackend {
    fn backend_id(&self) -> &'static str;
    fn generate_curriculum(
        &self,
        request: &CurriculumRequest,
    ) -> Result<(CurriculumBundle, TeacherCallRecord)>;
}

pub fn build_backend(config: &TeacherConfig) -> Box<dyn TeacherBackend> {
    match config.backend {
        TeacherBackendKind::Mock => Box::new(MockTeacherBackend::new(config.clone())),
        TeacherBackendKind::LocalLlm => Box::new(LocalLlmTeacherBackend::new(config.clone())),
    }
}

#[derive(Debug, Clone)]
pub struct MockTeacherBackend {
    config: TeacherConfig,
}

impl MockTeacherBackend {
    pub fn new(config: TeacherConfig) -> Self {
        Self { config }
    }
}

impl TeacherBackend for MockTeacherBackend {
    fn backend_id(&self) -> &'static str {
        "mock"
    }

    fn generate_curriculum(
        &self,
        request: &CurriculumRequest,
    ) -> Result<(CurriculumBundle, TeacherCallRecord)> {
        let started = Instant::now();
        let curriculum = build_mock_curriculum(request, self.backend_id());
        let record = TeacherCallRecord {
            teacher_call_id: Uuid::new_v4().to_string(),
            session_id: request.session_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            teacher_backend_id: self.backend_id().to_string(),
            operation: "generate_curriculum".to_string(),
            request_payload: serde_json::to_value(request)?,
            response_payload: serde_json::json!({
                "curriculum_id": curriculum.curriculum_id,
                "item_count": curriculum.items.len(),
                "domain_count": curriculum.domains.len(),
                "notes": curriculum.generation_notes,
            }),
            token_counts: Some(TokenCounts {
                input_tokens: 0,
                output_tokens: 0,
            }),
            latency_ms: started.elapsed().as_millis() as u64,
            model_config: serde_json::json!({
                "kind": "deterministic_mock"
            }),
            runtime_config: serde_json::to_value(&self.config.runtime)?,
            success: true,
            error: None,
        };
        Ok((curriculum, record))
    }
}

#[derive(Debug, Clone)]
pub struct LocalLlmTeacherBackend {
    config: TeacherConfig,
}

impl LocalLlmTeacherBackend {
    pub fn new(config: TeacherConfig) -> Self {
        Self { config }
    }

    fn client(&self) -> Result<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("failed to build local teacher HTTP client")
    }

    fn ensure_runtime_ready(&self) -> Result<RuntimeStatus> {
        let client = self.client()?;
        if self.endpoint_is_ready(&client) {
            return Ok(RuntimeStatus {
                endpoint_ready: true,
                launched_runtime: false,
                launch_pid: None,
            });
        }

        if !self.config.runtime.enabled {
            bail!(
                "local teacher endpoint {} is unavailable and runtime launching is disabled",
                self.config.runtime.endpoint
            );
        }

        let pid = self.start_runtime_process()?;
        self.wait_for_endpoint(&client)?;

        Ok(RuntimeStatus {
            endpoint_ready: true,
            launched_runtime: true,
            launch_pid: Some(pid),
        })
    }

    fn endpoint_is_ready(&self, client: &reqwest::blocking::Client) -> bool {
        let models_url = format!("{}/models", normalized_endpoint(&self.config.runtime.endpoint));
        client
            .get(models_url)
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    }

    fn wait_for_endpoint(&self, client: &reqwest::blocking::Client) -> Result<()> {
        let started = Instant::now();
        let timeout = Duration::from_secs(90);

        while started.elapsed() < timeout {
            if self.endpoint_is_ready(client) {
                return Ok(());
            }
            thread::sleep(Duration::from_secs(2));
        }

        bail!(
            "local teacher endpoint {} did not become ready within {} seconds",
            self.config.runtime.endpoint,
            timeout.as_secs()
        )
    }

    fn start_runtime_process(&self) -> Result<u32> {
        let workspace_root = std::env::current_dir().context("failed to resolve workspace root")?;
        let runtime_dir = resolve_workspace_path(&workspace_root, &self.config.runtime.runtime_path);
        let server_binary =
            resolve_workspace_path(&workspace_root, &self.config.runtime.server_binary);
        let model_path = resolve_workspace_path(&workspace_root, &self.config.local_model.model_path);
        let (host, port) = parse_endpoint_host_port(&self.config.runtime.endpoint)?;

        if !server_binary.is_file() {
            bail!("local teacher server binary not found at {}", server_binary.display());
        }
        if !model_path.is_file() {
            bail!("local teacher model not found at {}", model_path.display());
        }

        let mut command = Command::new(&server_binary);
        command
            .current_dir(&runtime_dir)
            .arg("-m")
            .arg(&model_path)
            .arg("-c")
            .arg(self.config.local_model.context_size.to_string())
            .arg("-ngl")
            .arg(self.config.local_model.gpu_layers.to_string())
            .arg("--host")
            .arg(&host)
            .arg("--port")
            .arg(port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        let child = command.spawn().with_context(|| {
            format!(
                "failed to launch local teacher runtime {}",
                server_binary.display()
            )
        })?;

        Ok(child.id())
    }

    fn request_outline(
        &self,
        request: &CurriculumRequest,
        client: &reqwest::blocking::Client,
    ) -> Result<LlmCompletionResponse> {
        let endpoint = format!("{}/completions", normalized_endpoint(&self.config.runtime.endpoint));
        let catalog = outline_catalog_reference();
        let prompt = format!(
            "You are the Janet School teacher.\n\
Return one compact JSON object and nothing else.\n\
Use only domain_id and concept_id values from the provided catalog.\n\
JSON shape:\n\
{{\"rationale\":\"...\",\"domains\":[{{\"domain_id\":\"...\",\"domain_name\":\"...\",\"concepts\":[{{\"concept_id\":\"...\",\"concept_name\":\"...\"}}]}}]}}\n\
Choose at most {target_domains} domains and at most {concepts_per_domain} concepts per selected domain.\n\n\
Curriculum request:\n{request_json}\n\n\
Catalog:\n{catalog_json}\n\n\
Final JSON:",
            target_domains = request.target_domain_count,
            concepts_per_domain = request.concepts_per_domain,
            request_json = serde_json::to_string(request)?,
            catalog_json = serde_json::to_string(&catalog)?,
        );
        let payload = serde_json::json!({
            "model": self.model_identifier(),
            "temperature": 0.2,
            "max_tokens": 600,
            "prompt": prompt
        });

        let response = client
            .post(endpoint)
            .json(&payload)
            .send()
            .context("failed to call local teacher endpoint")?;
        let status = response.status();
        let body = response.text().context("failed to read local teacher response")?;

        if status != StatusCode::OK {
            bail!("local teacher returned {}: {}", status, body);
        }

        serde_json::from_str(&body).context("failed to parse local teacher response JSON")
    }

    fn outline_from_response(&self, response: &LlmCompletionResponse) -> Result<CurriculumOutline> {
        let content = response
            .choices
            .first()
            .map(|choice| choice.text.clone())
            .ok_or_else(|| anyhow!("local teacher response did not include completion text"))?;
        let json = extract_json_object(&content)?;
        serde_json::from_str(&json)
            .with_context(|| format!("failed to parse local teacher outline JSON from: {}", json))
    }

    fn model_identifier(&self) -> String {
        Path::new(&self.config.local_model.model_path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| self.config.local_model.model_path.clone())
    }
}

impl TeacherBackend for LocalLlmTeacherBackend {
    fn backend_id(&self) -> &'static str {
        "local_llm"
    }

    fn generate_curriculum(
        &self,
        request: &CurriculumRequest,
    ) -> Result<(CurriculumBundle, TeacherCallRecord)> {
        let started = Instant::now();
        let client = self.client()?;
        let runtime_status = self.ensure_runtime_ready()?;
        let response = self.request_outline(request, &client)?;
        let outline = self.outline_from_response(&response)?;
        let curriculum = build_curriculum_from_outline(request, &outline, self.backend_id());

        let record = TeacherCallRecord {
            teacher_call_id: Uuid::new_v4().to_string(),
            session_id: request.session_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            teacher_backend_id: self.backend_id().to_string(),
            operation: "generate_curriculum".to_string(),
            request_payload: serde_json::to_value(request)?,
            response_payload: serde_json::json!({
                "outline": outline,
                "curriculum_id": curriculum.curriculum_id,
                "item_count": curriculum.items.len(),
                "domain_count": curriculum.domains.len(),
                "generation_notes": curriculum.generation_notes,
                "raw_response": response,
            }),
            token_counts: response.usage.as_ref().map(|usage| TokenCounts {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            }),
            latency_ms: started.elapsed().as_millis() as u64,
            model_config: serde_json::json!({
                "kind": "local_llm",
                "model_path": self.config.local_model.model_path,
                "context_size": self.config.local_model.context_size,
                "gpu_layers": self.config.local_model.gpu_layers,
            }),
            runtime_config: serde_json::json!({
                "enabled": self.config.runtime.enabled,
                "runtime_path": self.config.runtime.runtime_path,
                "server_binary": self.config.runtime.server_binary,
                "endpoint": self.config.runtime.endpoint,
                "endpoint_ready": runtime_status.endpoint_ready,
                "launched_runtime": runtime_status.launched_runtime,
                "launch_pid": runtime_status.launch_pid,
            }),
            success: true,
            error: None,
        };
        Ok((curriculum, record))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmCompletionResponse {
    choices: Vec<LlmCompletionChoice>,
    usage: Option<LlmUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmCompletionChoice {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeStatus {
    endpoint_ready: bool,
    launched_runtime: bool,
    launch_pid: Option<u32>,
}

fn normalized_endpoint(endpoint: &str) -> String {
    endpoint.trim_end_matches('/').to_string()
}

fn resolve_workspace_path(workspace_root: &Path, value: &str) -> PathBuf {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace_root.join(candidate)
    }
}

fn parse_endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    let url = reqwest::Url::parse(endpoint)
        .with_context(|| format!("failed to parse teacher endpoint {}", endpoint))?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("teacher endpoint is missing a host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(8080);

    let mut resolved = (host.as_str(), port)
        .to_socket_addrs()
        .with_context(|| format!("failed to resolve teacher endpoint host {}", host))?;
    if resolved.next().is_none() {
        bail!("teacher endpoint host {} did not resolve", host);
    }

    Ok((host, port))
}

fn extract_json_object(content: &str) -> Result<String> {
    let trimmed = content.trim();
    let unwrapped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|value| value.trim())
        .unwrap_or(trimmed);
    let unwrapped = unwrapped.strip_suffix("```").map(str::trim).unwrap_or(unwrapped);

    if unwrapped.starts_with('{') && unwrapped.ends_with('}') {
        if serde_json::from_str::<serde_json::Value>(unwrapped).is_ok() {
            return Ok(unwrapped.to_string());
        }
    }

    let start = unwrapped
        .find('{')
        .ok_or_else(|| anyhow!("local teacher response did not contain a JSON object"))?;

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in unwrapped[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start + offset;
                    return Ok(unwrapped[start..=end].to_string());
                }
            }
            _ => {}
        }
    }

    bail!("local teacher response did not contain a complete JSON object")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    use tiny_http::{Method, Response, Server, StatusCode};

    use crate::config::{LocalModelConfig, RuntimeConfig};

    #[test]
    fn local_backend_generates_curriculum_from_ready_endpoint() {
        let server = spawn_fake_local_teacher_server();
        let config = TeacherConfig {
            backend: TeacherBackendKind::LocalLlm,
            runtime: RuntimeConfig {
                enabled: false,
                runtime_path: "runtime".to_string(),
                server_binary: "runtime/llama-server.exe".to_string(),
                endpoint: server.endpoint.clone(),
            },
            local_model: LocalModelConfig {
                model_path: "models/test-model.gguf".to_string(),
                context_size: 4096,
                gpu_layers: 0,
            },
        };
        let backend = LocalLlmTeacherBackend::new(config);
        let request = CurriculumRequest::from_size_hint(
            "session-local".to_string(),
            "smoke".to_string(),
            "smoke",
        );

        let (curriculum, record) = backend
            .generate_curriculum(&request)
            .expect("local backend should generate curriculum");

        let summary = curriculum.summary();
        assert_eq!(record.teacher_backend_id, "local_llm");
        assert_eq!(record.operation, "generate_curriculum");
        assert!(record.success);
        assert_eq!(
            record
                .runtime_config
                .get("endpoint_ready")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            record
                .runtime_config
                .get("launched_runtime")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            record
                .token_counts
                .as_ref()
                .map(|counts| (counts.input_tokens, counts.output_tokens)),
            Some((123, 234))
        );
        assert_eq!(summary.domain_count, 5);
        assert_eq!(summary.concept_count, 10);
        assert_eq!(summary.item_count, 30);
        assert_eq!(summary.probe_count, 10);
    }

    #[test]
    fn local_backend_fails_when_endpoint_unavailable_and_runtime_launch_disabled() {
        let port = unused_local_port();
        let config = TeacherConfig {
            backend: TeacherBackendKind::LocalLlm,
            runtime: RuntimeConfig {
                enabled: false,
                runtime_path: "runtime".to_string(),
                server_binary: "runtime/llama-server.exe".to_string(),
                endpoint: format!("http://127.0.0.1:{port}/v1"),
            },
            local_model: LocalModelConfig {
                model_path: "models/test-model.gguf".to_string(),
                context_size: 4096,
                gpu_layers: 0,
            },
        };
        let backend = LocalLlmTeacherBackend::new(config);
        let request = CurriculumRequest::from_size_hint(
            "session-local-fail".to_string(),
            "smoke".to_string(),
            "smoke",
        );

        let error = backend
            .generate_curriculum(&request)
            .expect_err("local backend should fail when endpoint is unavailable");
        assert!(error
            .to_string()
            .contains("runtime launching is disabled"));
    }

    struct FakeLocalTeacherServer {
        endpoint: String,
        join_handle: thread::JoinHandle<()>,
    }

    impl Drop for FakeLocalTeacherServer {
        fn drop(&mut self) {
            let shutdown_url = self.endpoint.replace("/v1", "/shutdown");
            let _ = reqwest::blocking::get(shutdown_url);
            let handle = std::mem::replace(&mut self.join_handle, thread::spawn(|| {}));
            let _ = handle.join();
        }
    }

    fn spawn_fake_local_teacher_server() -> FakeLocalTeacherServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");
        drop(listener);

        let endpoint = format!("http://127.0.0.1:{}/v1", address.port());
        let server = Server::http(format!("127.0.0.1:{}", address.port()))
            .expect("tiny http server should start");
        let join_handle = thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                let method = request.method().clone();

                if method == Method::Get && url == "/shutdown" {
                    let _ = request.respond(Response::empty(StatusCode(200)));
                    break;
                }

                if method == Method::Get && url == "/v1/models" {
                    let response = Response::from_string("{\"data\":[]}")
                        .with_status_code(StatusCode(200));
                    let _ = request.respond(response);
                    continue;
                }

                if method == Method::Post && url == "/v1/completions" {
                    let body = serde_json::json!({
                        "choices": [{
                            "text": "{\"rationale\":\"local test outline\",\"domains\":[{\"domain_id\":\"attention_discrimination\",\"domain_name\":\"Attention and Discrimination\",\"concepts\":[{\"concept_id\":\"attention_discrimination_visual_notice\",\"concept_name\":\"visual notice\"},{\"concept_id\":\"attention_discrimination_signal_pickout\",\"concept_name\":\"signal pickout\"}]},{\"domain_id\":\"matching_difference\",\"domain_name\":\"Matching and Difference\",\"concepts\":[{\"concept_id\":\"matching_difference_same_vs_different\",\"concept_name\":\"same vs different\"},{\"concept_id\":\"matching_difference_exact_match\",\"concept_name\":\"exact match\"}]},{\"domain_id\":\"sequencing_order\",\"domain_name\":\"Sequencing and Order\",\"concepts\":[{\"concept_id\":\"sequencing_order_before_after\",\"concept_name\":\"before after\"},{\"concept_id\":\"sequencing_order_first_last\",\"concept_name\":\"first last\"}]},{\"domain_id\":\"quantity_comparison\",\"domain_name\":\"Quantity and Comparison\",\"concepts\":[{\"concept_id\":\"quantity_comparison_more_less\",\"concept_name\":\"more less\"},{\"concept_id\":\"quantity_comparison_equal_compare\",\"concept_name\":\"equal compare\"}]},{\"domain_id\":\"spatial_reasoning\",\"domain_name\":\"Spatial Reasoning\",\"concepts\":[{\"concept_id\":\"spatial_reasoning_left_right\",\"concept_name\":\"left right\"},{\"concept_id\":\"spatial_reasoning_inside_outside\",\"concept_name\":\"inside outside\"}]}]}"
                        }],
                        "usage": {
                            "prompt_tokens": 123,
                            "completion_tokens": 234
                        }
                    });
                    let response = Response::from_string(body.to_string())
                        .with_status_code(StatusCode(200));
                    let _ = request.respond(response);
                    continue;
                }

                let _ = request.respond(Response::empty(StatusCode(404)));
            }
        });

        FakeLocalTeacherServer { endpoint, join_handle }
    }

    fn unused_local_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .expect("listener should bind")
            .local_addr()
            .expect("listener should have local addr")
            .port()
    }
}

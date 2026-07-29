use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

use crate::analysis::AnalysisReport;
use crate::config::{
    AppConfig, CurriculumSizeHint, RunMode, SkillApprovals, TeacherBackendKind,
    write_app_metadata, write_skill_approvals, write_teacher_config,
};
use crate::curriculum::{CurriculumBundle, CurriculumValidationReport};
use crate::mcm::McmTraceEvent;
use crate::memory::MemoryEvent;
use crate::session::{RunControlSignal, SessionProgressUpdate, SessionService, SessionSummary};
use crate::skills::SkillEvent;
use crate::teacher::{TeacherCallRecord, TokenCounts};
use crate::telemetry::{AnomalyEvent, InteractionEvent, RefusalEvent};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiShell {
    pub shell_name: String,
    pub shell_type: String,
    pub index_path: String,
    pub script_path: String,
    pub bridge_script_path: String,
    pub styles_path: String,
    pub state_path: String,
    pub backend_mode: String,
    pub status: String,
    pub launch_command: String,
    pub local_url: String,
}

impl GuiShell {
    pub fn from_workspace(root: PathBuf) -> Self {
        let web_dir = root.join("web");
        Self {
            shell_name: "Janet School GUI Shell".to_string(),
            shell_type: "static_webview_ready".to_string(),
            index_path: web_dir.join("index.html").to_string_lossy().to_string(),
            script_path: web_dir.join("app.js").to_string_lossy().to_string(),
            bridge_script_path: web_dir.join("bridge.js").to_string_lossy().to_string(),
            styles_path: web_dir.join("styles.css").to_string_lossy().to_string(),
            state_path: web_dir.join("gui-state.json").to_string_lossy().to_string(),
            backend_mode: "headless_capable".to_string(),
            status: "gui_state_driven".to_string(),
            launch_command: "cargo run -- serve-gui".to_string(),
            local_url: "http://127.0.0.1:8787".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiState {
    pub app_name: String,
    pub version: String,
    pub backend_mode: String,
    pub teacher_backend: String,
    pub configured_teacher_backend: String,
    pub splash: GuiSplashConfig,
    pub setup_snapshot: GuiSetupSnapshot,
    pub skill_snapshot: GuiSkillSnapshot,
    pub control_surface: GuiControlSurface,
    pub latest_session: Option<GuiSessionCard>,
    pub recent_sessions: Vec<GuiSessionCard>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSetupSnapshot {
    pub environment: String,
    pub configured_teacher_backend: String,
    pub configured_run_mode: String,
    pub curriculum_size_hint: String,
    pub sessions_dir: String,
    pub aggregated_dir: String,
    pub runtime_enabled: bool,
    pub runtime_path: String,
    pub runtime_path_exists: bool,
    pub server_binary: String,
    pub server_binary_exists: bool,
    pub endpoint: String,
    pub endpoint_ready: bool,
    pub model_path: String,
    pub model_path_exists: bool,
    pub context_size: u32,
    pub gpu_layers: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSplashConfig {
    pub show_splash: bool,
    pub duration_ms: u64,
    pub asset_path: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSessionCard {
    pub session_id: String,
    pub run_id: String,
    pub run_mode: String,
    pub teacher_backend_id: String,
    pub completed_at: Option<String>,
    pub notes: Vec<String>,
    pub skill_run_snapshot: Option<GuiSessionSkillRunSnapshot>,
    pub curriculum_stats: serde_json::Value,
    pub curriculum_preview: Option<GuiCurriculumPreview>,
    pub interaction_stats: serde_json::Value,
    pub memory_stats: serde_json::Value,
    pub refusal_stats: serde_json::Value,
    pub anomaly_stats: serde_json::Value,
    pub analysis_snapshot: Option<GuiAnalysisSnapshot>,
    pub teacher_snapshot: Option<GuiTeacherSnapshot>,
    pub telemetry_preview: Vec<GuiTelemetryPreviewRow>,
    pub comparison_items: Vec<GuiComparisonItemOutcome>,
    pub artifacts: Vec<GuiArtifactDescriptor>,
    pub root_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCurriculumPreview {
    pub generation_notes: Vec<String>,
    pub warnings: Vec<String>,
    pub domain_summaries: Vec<GuiCurriculumDomainSummary>,
    pub item_mix: Vec<GuiCountMetric>,
    pub sample_items: Vec<GuiCurriculumItemPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCurriculumDomainSummary {
    pub domain_id: String,
    pub name: String,
    pub concept_count: usize,
    pub item_count: usize,
    pub probe_count: usize,
    pub concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCurriculumItemPreview {
    pub item_id: String,
    pub domain_id: String,
    pub concept_id: String,
    pub item_type: String,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub expected_skills: Vec<String>,
    pub novelty_class: String,
    pub probe_role: String,
    pub boundary_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCountMetric {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSessionSkillRunSnapshot {
    pub selection_mode: String,
    pub approved_count: usize,
    pub total_skill_count: usize,
    pub approved_skill_ids: Vec<String>,
    pub blocked_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSkillSnapshot {
    pub approvals_version: String,
    pub manifest_version: String,
    pub approved_count: usize,
    pub blocked_count: usize,
    pub entries: Vec<GuiSkillEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSkillEntry {
    pub skill_id: String,
    pub description: String,
    pub deterministic: bool,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiAnalysisSnapshot {
    pub confirmed_count: usize,
    pub boundary_count: usize,
    pub emergent_count: usize,
    pub unknown_count: usize,
    pub repeated_anomaly_cluster_count: usize,
    pub category_mismatch_cluster_count: usize,
    pub confirmed_summaries: Vec<String>,
    pub boundary_summaries: Vec<String>,
    pub emergent_summaries: Vec<String>,
    pub caution_notes: Vec<String>,
    pub recommended_next_probes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiTeacherSnapshot {
    pub teacher_backend_id: String,
    pub operation: String,
    pub timestamp: String,
    pub success: bool,
    pub latency_ms: u64,
    pub token_counts: Option<TokenCounts>,
    pub launched_runtime: bool,
    pub endpoint_ready: bool,
    pub model_kind: Option<String>,
    pub model_path: Option<String>,
    pub rationale: Option<String>,
    pub selected_domain_ids: Vec<String>,
    pub selected_concept_ids: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiArtifactDescriptor {
    pub label: String,
    pub file_name: String,
    pub absolute_path: String,
    pub relative_path: String,
    pub content_type: String,
    pub download_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiTelemetryPreviewRow {
    pub item_id: String,
    pub item_type: String,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub janet_answer: Option<String>,
    pub correctness_judgment: String,
    pub structure_fit: String,
    pub anomaly_flags: Vec<String>,
    pub final_mode: Option<String>,
    pub uncertainty_state: Option<String>,
    pub executed_skill: Option<String>,
    pub candidate_skills: Vec<String>,
    pub approved_skills: Vec<String>,
    pub blocked_skills: Vec<String>,
    pub policy_checks: Vec<String>,
    pub reasoning_steps: Vec<String>,
    pub memory_reads: Vec<String>,
    pub refusal_reason: Option<String>,
    pub refusal_next_steps: Vec<String>,
    pub anomaly_explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiComparisonItemOutcome {
    pub item_id: String,
    pub item_type: String,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub janet_answer: Option<String>,
    pub correctness_judgment: String,
    pub structure_fit: String,
    pub executed_skill: Option<String>,
    pub refusal_reason: Option<String>,
    pub anomaly_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiControlSurface {
    pub bridge_mode: String,
    pub actions: Vec<GuiActionDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiActionDescriptor {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub supports_teacher_backend: bool,
    pub supports_session_name: bool,
    pub supported_teacher_backends: Vec<String>,
    pub command_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiActionRequest {
    pub action: String,
    pub session_name: Option<String>,
    pub teacher_backend: Option<String>,
    pub selected_skill_ids: Option<Vec<String>>,
    pub run_mode: Option<String>,
    pub curriculum_size_hint: Option<String>,
    pub model_path: Option<String>,
    pub endpoint: Option<String>,
    pub sessions_dir: Option<String>,
    pub aggregated_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCompareExportRequest {
    pub file_name: String,
    pub content: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiSessionExportRequest {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiCompareExportSaved {
    pub file_name: String,
    pub absolute_path: String,
    pub relative_path: String,
    pub download_path: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiOpenFolderResult {
    pub opened: bool,
    pub absolute_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiServerDescriptor {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiBridgeStatus {
    pub active_job: Option<GuiActionJob>,
    pub recent_jobs: Vec<GuiActionJob>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiActionJob {
    pub job_id: String,
    pub action: String,
    pub teacher_backend: Option<String>,
    pub session_name: Option<String>,
    pub state: String,
    pub cancel_requested: bool,
    pub pause_requested: bool,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result_summary: Option<String>,
    pub progress: Option<SessionProgressUpdate>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuiActionAccepted {
    pub accepted: bool,
    pub job: GuiActionJob,
}

#[derive(Debug, Default)]
struct BridgeRuntimeState {
    jobs: Vec<GuiActionJob>,
}

pub fn sync_gui_state(config: &AppConfig, workspace_root: &Path) -> Result<GuiState> {
    let sessions_root = workspace_root.join(&config.session.sessions_dir);
    let web_root = workspace_root.join(&config.app.web_dir);
    fs::create_dir_all(&web_root)
        .with_context(|| format!("failed to create web dir {}", web_root.display()))?;

    let recent_sessions = collect_recent_sessions(&sessions_root)?;
    let latest_session = recent_sessions.first().cloned();
    let teacher_backend = latest_session
        .as_ref()
        .map(|session| session.teacher_backend_id.clone())
        .unwrap_or_else(|| config.teacher.backend.as_str().to_string());
    let state = GuiState {
        app_name: config.app.app_name.clone(),
        version: config.app.version.clone(),
        backend_mode: "headless_capable".to_string(),
        teacher_backend,
        configured_teacher_backend: config.teacher.backend.as_str().to_string(),
        splash: GuiSplashConfig {
            show_splash: config.app.show_splash,
            duration_ms: config.app.splash_duration_ms,
            asset_path: "/assets/fmi-splash-wordmark.png".to_string(),
            label: "Fractal Media Infrastructure".to_string(),
        },
        setup_snapshot: build_setup_snapshot(config, workspace_root),
        skill_snapshot: build_skill_snapshot(config),
        control_surface: build_control_surface(),
        latest_session,
        recent_sessions,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    fs::write(
        web_root.join("gui-state.json"),
        serde_json::to_vec_pretty(&state)?,
    )
    .with_context(|| "failed to write gui-state.json".to_string())?;

    Ok(state)
}

fn load_gui_config(config_dir: &Path, workspace_root: &Path) -> Result<AppConfig> {
    Ok(AppConfig::load_from_dir(config_dir)?.resolved_against(workspace_root))
}

pub fn serve_gui_bridge(
    config_dir: &Path,
    workspace_root: &Path,
    host: &str,
    port: u16,
    open_browser: bool,
) -> Result<()> {
    let address = format!("{host}:{port}");
    let server = Server::http(&address)
        .map_err(|error| anyhow!("failed to start GUI server on {}: {}", address, error))?;
    let runtime_state = Arc::new(Mutex::new(BridgeRuntimeState::default()));
    if open_browser {
        let browser_host = match host {
            "0.0.0.0" => "127.0.0.1",
            "::" => "127.0.0.1",
            _ => host,
        };
        let local_url = format!("http://{browser_host}:{port}");
        if let Err(error) = open_browser_url(&local_url) {
            eprintln!("browser auto-launch skipped: {error}");
        }
    }

    for request in server.incoming_requests() {
        if let Err(error) = handle_http_request(
            request,
            config_dir.to_path_buf(),
            workspace_root.to_path_buf(),
            runtime_state.clone(),
        ) {
            eprintln!("gui bridge request failed: {error:#}");
        }
    }

    Ok(())
}

fn open_browser_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(url)
            .spawn()
            .with_context(|| format!("failed to open browser for {}", url))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .with_context(|| format!("failed to open browser for {}", url))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .with_context(|| format!("failed to open browser for {}", url))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(anyhow!("automatic browser launch is not supported on this platform"))
}

fn handle_http_request(
    mut request: Request,
    config_dir: PathBuf,
    workspace_root: PathBuf,
    runtime_state: Arc<Mutex<BridgeRuntimeState>>,
) -> Result<()> {
    let method = request.method().clone();
    let url = request.url().to_string();

    match (method, url.as_str()) {
        (Method::Get, "/api/state") => {
            let config = load_gui_config(&config_dir, &workspace_root)?;
            let state = with_live_bridge_state(sync_gui_state(&config, &workspace_root)?);
            respond_json(request, StatusCode(200), &state)
        }
        (Method::Get, "/api/bridge-status") => {
            let status = build_bridge_status(&runtime_state)?;
            respond_json(request, StatusCode(200), &status)
        }
        (Method::Post, "/api/gui-action") => {
            let payload = read_json_body::<GuiActionRequest>(request.as_reader())?;
            let accepted = enqueue_gui_action(payload, config_dir, workspace_root, runtime_state)?;
            respond_json(request, StatusCode(202), &accepted)
        }
        (Method::Post, "/api/compare-export") => {
            let payload = read_json_body::<GuiCompareExportRequest>(request.as_reader())?;
            let saved = save_compare_export(&workspace_root, payload)?;
            respond_json(request, StatusCode(201), &saved)
        }
        (Method::Post, "/api/session-export") => {
            let payload = read_json_body::<GuiSessionExportRequest>(request.as_reader())?;
            let config = load_gui_config(&config_dir, &workspace_root)?;
            let saved = save_session_export_bundle(&workspace_root, &config, payload)?;
            respond_json(request, StatusCode(201), &saved)
        }
        (Method::Post, "/api/open-session-folder") => {
            let payload = read_json_body::<GuiSessionExportRequest>(request.as_reader())?;
            let config = load_gui_config(&config_dir, &workspace_root)?;
            let opened = open_session_folder(&config, payload)?;
            respond_json(request, StatusCode(200), &opened)
        }
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, workspace_root.join("web").join("index.html"), "text/html; charset=utf-8")
        }
        (Method::Get, "/app.js") => {
            serve_static_file(request, workspace_root.join("web").join("app.js"), "application/javascript; charset=utf-8")
        }
        (Method::Get, "/bridge.js") => {
            serve_static_file(request, workspace_root.join("web").join("bridge.js"), "application/javascript; charset=utf-8")
        }
        (Method::Get, "/styles.css") => {
            serve_static_file(request, workspace_root.join("web").join("styles.css"), "text/css; charset=utf-8")
        }
        (Method::Get, _) if url.starts_with("/assets/") => {
            serve_asset_file(request, &workspace_root, &url)
        }
        (Method::Get, "/gui-state.json") => {
            let config = load_gui_config(&config_dir, &workspace_root)?;
            let state = with_live_bridge_state(sync_gui_state(&config, &workspace_root)?);
            respond_json(request, StatusCode(200), &state)
        }
        (Method::Get, _) if url.starts_with("/artifacts/") => {
            serve_session_artifact(request, &config_dir, &workspace_root, &url)
        }
        (Method::Get, _) if url.starts_with("/compare-exports/") => {
            serve_compare_export(request, &workspace_root, &url)
        }
        (Method::Get, _) if url.starts_with("/session-exports/") => {
            serve_session_export(request, &workspace_root, &url)
        }
        _ => respond_text(request, StatusCode(404), "not found", "text/plain; charset=utf-8"),
    }
}

fn with_live_bridge_state(mut state: GuiState) -> GuiState {
    state.control_surface.bridge_mode = "http_bridge_live".to_string();
    state
}

fn execute_gui_action_request(
    config_dir: &Path,
    workspace_root: &Path,
    payload: GuiActionRequest,
) -> Result<serde_json::Value> {
    match payload.action.as_str() {
        "sync_state" => {
            let config = load_gui_config(config_dir, workspace_root)?;
            let state = sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(state)?)
        }
        "save_setup" => {
            let config = persist_setup_overrides(config_dir, workspace_root, payload)?;
            let state = sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(state)?)
        }
        "generate_curriculum" => {
            let config = load_config_with_overrides(config_dir, workspace_root, &payload)?;
            let service = SessionService::new(config);
            let generated = service.initialize_and_generate_curriculum(payload.session_name)?;
            let config = load_gui_config(config_dir, workspace_root)?;
            sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(generated)?)
        }
        "run_session" => {
            let config = load_config_with_overrides(config_dir, workspace_root, &payload)?;
            let service = SessionService::new(config);
            let completed = service.run_generated_curriculum_session(payload.session_name)?;
            let config = load_gui_config(config_dir, workspace_root)?;
            sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(completed)?)
        }
        "update_skill_approvals" => {
            let updated = update_skill_approvals(config_dir, payload.selected_skill_ids.unwrap_or_default())?;
            let config = load_gui_config(config_dir, workspace_root)?;
            sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(updated)?)
        }
        "stop_run" | "pause_run" | "resume_run" => {
            bail!("run control actions must be handled by the live GUI bridge")
        }
        other => bail!("unsupported gui action: {}", other),
    }
}

fn enqueue_gui_action(
    payload: GuiActionRequest,
    config_dir: PathBuf,
    workspace_root: PathBuf,
    runtime_state: Arc<Mutex<BridgeRuntimeState>>,
) -> Result<GuiActionAccepted> {
    if payload.action == "stop_run" {
        return request_stop_for_active_job(runtime_state);
    }
    if payload.action == "pause_run" {
        return request_pause_for_active_job(runtime_state);
    }
    if payload.action == "resume_run" {
        return request_resume_for_active_job(runtime_state);
    }

    let job = GuiActionJob {
        job_id: Uuid::new_v4().to_string(),
        action: payload.action.clone(),
        teacher_backend: payload.teacher_backend.clone(),
        session_name: payload.session_name.clone(),
        state: "queued".to_string(),
        cancel_requested: false,
        pause_requested: false,
        created_at: Utc::now().to_rfc3339(),
        started_at: None,
        finished_at: None,
        result_summary: None,
        progress: None,
        error: None,
    };

    {
        let mut state = runtime_state
            .lock()
            .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
        state.jobs.insert(0, job.clone());
        state.jobs.truncate(12);
    }

    let job_id = job.job_id.clone();
    thread::spawn(move || {
        if let Err(error) = update_job_state(&runtime_state, &job_id, |job| {
            job.state = "running".to_string();
            job.started_at = Some(Utc::now().to_rfc3339());
        }) {
            eprintln!("failed to mark gui job running: {error:#}");
            return;
        }

        let result = execute_gui_action_request_with_progress(
            &config_dir,
            &workspace_root,
            payload,
            || current_run_control_signal(&runtime_state, &job_id).unwrap_or(RunControlSignal::Continue),
            |progress| {
                let summary = summarize_progress(&progress);
                let _ = update_job_state(&runtime_state, &job_id, |job| {
                    if job.cancel_requested && job.state == "running" {
                        job.state = "cancelling".to_string();
                    }
                    if progress.phase == "session_paused" {
                        job.state = "paused".to_string();
                    } else if job.pause_requested && job.state == "running" {
                        job.state = "pausing".to_string();
                    } else if !job.pause_requested
                        && (job.state == "paused" || job.state == "pausing")
                        && progress.phase != "session_stopped"
                    {
                        job.state = "running".to_string();
                    }
                    job.progress = Some(progress.clone());
                    job.result_summary = Some(summary.clone());
                });
            },
        );
        let update_result = update_job_state(&runtime_state, &job_id, |job| {
            job.finished_at = Some(Utc::now().to_rfc3339());
            match result {
                Ok(ref value) => {
                    let completion_status = value
                        .get("completion_status")
                        .and_then(|entry| entry.as_str())
                        .unwrap_or("completed");
                    job.state = if completion_status == "stopped" {
                        "stopped".to_string()
                    } else {
                        "completed".to_string()
                    };
                    job.result_summary = Some(summarize_action_result(value));
                    job.error = None;
                }
                Err(ref error) => {
                    job.state = "failed".to_string();
                    job.result_summary = None;
                    job.error = Some(format!("{error:#}"));
                }
            }
        });

        if let Err(error) = update_result {
            eprintln!("failed to finalize gui job: {error:#}");
        }
    });

    Ok(GuiActionAccepted {
        accepted: true,
        job,
    })
}

fn update_job_state<F>(
    runtime_state: &Arc<Mutex<BridgeRuntimeState>>,
    job_id: &str,
    mut update: F,
) -> Result<()>
where
    F: FnMut(&mut GuiActionJob),
{
    let mut state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let job = state
        .jobs
        .iter_mut()
        .find(|job| job.job_id == job_id)
        .ok_or_else(|| anyhow!("bridge job {} not found", job_id))?;
    update(job);
    Ok(())
}

fn current_run_control_signal(
    runtime_state: &Arc<Mutex<BridgeRuntimeState>>,
    job_id: &str,
) -> Result<RunControlSignal> {
    let state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let job = state
        .jobs
        .iter()
        .find(|job| job.job_id == job_id)
        .ok_or_else(|| anyhow!("bridge job {} not found", job_id))?;

    if job.cancel_requested {
        Ok(RunControlSignal::Stop)
    } else if job.pause_requested {
        Ok(RunControlSignal::Pause)
    } else {
        Ok(RunControlSignal::Continue)
    }
}

fn request_stop_for_active_job(
    runtime_state: Arc<Mutex<BridgeRuntimeState>>,
) -> Result<GuiActionAccepted> {
    let mut state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let job = state
        .jobs
        .iter_mut()
        .find(|job| job.state == "queued" || job.state == "running" || job.state == "cancelling")
        .ok_or_else(|| anyhow!("no active bridge job is available to stop"))?;

    job.cancel_requested = true;
    if job.state == "running" {
        job.state = "cancelling".to_string();
    }
    job.result_summary = Some("Stop requested. Waiting for the active run loop to halt cleanly.".to_string());

    Ok(GuiActionAccepted {
        accepted: true,
        job: job.clone(),
    })
}

fn request_pause_for_active_job(
    runtime_state: Arc<Mutex<BridgeRuntimeState>>,
) -> Result<GuiActionAccepted> {
    let mut state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let job = state
        .jobs
        .iter_mut()
        .find(|job| job.state == "queued" || job.state == "running" || job.state == "pausing")
        .ok_or_else(|| anyhow!("no active bridge job is available to pause"))?;

    job.pause_requested = true;
    if job.state == "running" || job.state == "queued" {
        job.state = "pausing".to_string();
    }
    job.result_summary =
        Some("Pause requested. Waiting for the active run loop to yield cleanly.".to_string());

    Ok(GuiActionAccepted {
        accepted: true,
        job: job.clone(),
    })
}

fn request_resume_for_active_job(
    runtime_state: Arc<Mutex<BridgeRuntimeState>>,
) -> Result<GuiActionAccepted> {
    let mut state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let job = state
        .jobs
        .iter_mut()
        .find(|job| job.state == "paused" || job.state == "pausing")
        .ok_or_else(|| anyhow!("no paused bridge job is available to resume"))?;

    job.pause_requested = false;
    job.state = "running".to_string();
    job.result_summary = Some("Resume requested. Session execution is continuing.".to_string());

    Ok(GuiActionAccepted {
        accepted: true,
        job: job.clone(),
    })
}

fn build_bridge_status(runtime_state: &Arc<Mutex<BridgeRuntimeState>>) -> Result<GuiBridgeStatus> {
    let state = runtime_state
        .lock()
        .map_err(|_| anyhow!("failed to lock bridge runtime state"))?;
    let active_job = state
        .jobs
        .iter()
        .find(|job| {
            matches!(
                job.state.as_str(),
                "queued" | "running" | "cancelling" | "pausing" | "paused"
            )
        })
        .cloned();
    let recent_jobs = state.jobs.iter().take(5).cloned().collect::<Vec<_>>();

    Ok(GuiBridgeStatus {
        active_job,
        recent_jobs,
        generated_at: Utc::now().to_rfc3339(),
    })
}

fn summarize_action_result(value: &serde_json::Value) -> String {
    if let Some(completion_status) = value.get("completion_status").and_then(|entry| entry.as_str())
        && completion_status == "stopped"
    {
        let total = value
            .get("run_stats")
            .and_then(|stats| stats.get("total_items"))
            .and_then(|count| count.as_u64())
            .unwrap_or(0);
        return format!("run stopped after {} items; partial artifacts were preserved", total);
    }

    if let Some(run_stats) = value.get("run_stats") {
        let total = run_stats
            .get("total_items")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let correct = run_stats
            .get("correct_count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        return format!("run completed: {} items, {} correct", total, correct);
    }

    if let Some(summary) = value.get("curriculum_summary") {
        let items = summary
            .get("item_count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let domains = summary
            .get("domain_count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        return format!("curriculum generated: {} items across {} domains", items, domains);
    }

    "action completed".to_string()
}

fn summarize_progress(progress: &SessionProgressUpdate) -> String {
    match progress.phase.as_str() {
        "session_initialized" => "session initialized".to_string(),
        "curriculum_generated" => progress.message.clone(),
        "session_running" => progress.message.clone(),
        "item_executed" => format!(
            "{} | correct={} incorrect={} refusals={} anomalies={}",
            progress.message,
            progress.correct_count,
            progress.incorrect_count,
            progress.refusal_count,
            progress.anomaly_count
        ),
        "session_paused" => progress.message.clone(),
        "session_stopped" => progress.message.clone(),
        "session_completed" => progress.message.clone(),
        _ => progress.message.clone(),
    }
}

fn execute_gui_action_request_with_progress<F>(
    config_dir: &Path,
    workspace_root: &Path,
    payload: GuiActionRequest,
    run_control: impl FnMut() -> RunControlSignal,
    mut on_progress: F,
) -> Result<serde_json::Value>
where
    F: FnMut(SessionProgressUpdate),
{
    match payload.action.as_str() {
        "sync_state" => execute_gui_action_request(config_dir, workspace_root, payload),
        "generate_curriculum" => {
            let config = load_config_with_overrides(config_dir, workspace_root, &payload)?;
            let service = SessionService::new(config);
            let generated =
                service.initialize_and_generate_curriculum_with_progress(payload.session_name, |progress| {
                    on_progress(progress);
                })?;
            let config = load_gui_config(config_dir, workspace_root)?;
            sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(generated)?)
        }
        "run_session" => {
            let config = load_config_with_overrides(config_dir, workspace_root, &payload)?;
            let service = SessionService::new(config);
            let completed =
                service.run_generated_curriculum_session_with_control(
                    payload.session_name,
                    |progress| {
                        on_progress(progress);
                    },
                    run_control,
                )?;
            let config = load_gui_config(config_dir, workspace_root)?;
            sync_gui_state(&config, workspace_root)?;
            Ok(serde_json::to_value(completed)?)
        }
        _ => execute_gui_action_request(config_dir, workspace_root, payload),
    }
}

fn load_config_with_overrides(
    config_dir: &Path,
    workspace_root: &Path,
    payload: &GuiActionRequest,
) -> Result<AppConfig> {
    let mut config = load_gui_config(config_dir, workspace_root)?;
    apply_setup_overrides(&mut config, payload)?;
    Ok(config.resolved_against(workspace_root))
}

fn apply_setup_overrides(config: &mut AppConfig, payload: &GuiActionRequest) -> Result<()> {
    if let Some(backend) = payload.teacher_backend.as_deref() {
        config.teacher.backend = parse_teacher_backend(backend)?;
    }
    if let Some(run_mode) = payload.run_mode.as_deref() {
        config.session.default_run_mode = parse_run_mode(run_mode)?;
        config.app.session.default_run_mode = parse_run_mode(run_mode)?;
    }
    if let Some(size_hint) = payload.curriculum_size_hint.as_deref() {
        config.session.curriculum_size_hint = parse_curriculum_size_hint(size_hint)?;
        config.app.session.curriculum_size_hint = parse_curriculum_size_hint(size_hint)?;
    }
    if let Some(model_path) = payload.model_path.as_ref() {
        config.teacher.local_model.model_path = model_path.clone();
    }
    if let Some(endpoint) = payload.endpoint.as_ref() {
        config.teacher.runtime.endpoint = endpoint.clone();
    }
    if let Some(sessions_dir) = payload.sessions_dir.as_ref() {
        config.session.sessions_dir = sessions_dir.clone();
        config.app.session.sessions_dir = sessions_dir.clone();
    }
    if let Some(aggregated_dir) = payload.aggregated_dir.as_ref() {
        config.session.aggregated_dir = aggregated_dir.clone();
        config.app.session.aggregated_dir = aggregated_dir.clone();
    }
    Ok(())
}

fn persist_setup_overrides(
    config_dir: &Path,
    workspace_root: &Path,
    payload: GuiActionRequest,
) -> Result<AppConfig> {
    let mut config = AppConfig::load_from_dir(config_dir)?;
    apply_setup_overrides(&mut config, &payload)?;
    write_app_metadata(config_dir, &config.app)?;
    write_teacher_config(config_dir, &config.teacher)?;
    Ok(config.resolved_against(workspace_root))
}

fn parse_teacher_backend(value: &str) -> Result<TeacherBackendKind> {
    match value {
        "mock" => Ok(TeacherBackendKind::Mock),
        "local_llm" | "local-llm" => Ok(TeacherBackendKind::LocalLlm),
        other => bail!("unsupported teacher backend: {}", other),
    }
}

fn parse_run_mode(value: &str) -> Result<RunMode> {
    match value {
        "smoke" => Ok(RunMode::Smoke),
        "full" => Ok(RunMode::Full),
        "analysis_only" | "analysis-only" => Ok(RunMode::AnalysisOnly),
        other => bail!("unsupported run mode: {}", other),
    }
}

fn parse_curriculum_size_hint(value: &str) -> Result<CurriculumSizeHint> {
    match value {
        "tiny_fixture" | "tiny-fixture" => Ok(CurriculumSizeHint::TinyFixture),
        "smoke" => Ok(CurriculumSizeHint::Smoke),
        "full" => Ok(CurriculumSizeHint::Full),
        other => bail!("unsupported curriculum size hint: {}", other),
    }
}

fn read_json_body<T: for<'de> Deserialize<'de>>(reader: &mut dyn Read) -> Result<T> {
    let mut body = String::new();
    reader.read_to_string(&mut body)?;
    Ok(serde_json::from_str(&body)?)
}

fn serve_static_file(request: Request, path: PathBuf, content_type: &str) -> Result<()> {
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read static file {}", path.display()))?;
    let response = Response::from_data(bytes).with_header(content_type_header(content_type)?);
    request.respond(response)?;
    Ok(())
}

fn serve_session_artifact(
    request: Request,
    config_dir: &Path,
    workspace_root: &Path,
    url: &str,
) -> Result<()> {
    let config = load_gui_config(config_dir, workspace_root)?;
    let Some(path) = resolve_artifact_request_path(&config, url)? else {
        return respond_text(request, StatusCode(404), "artifact not found", "text/plain; charset=utf-8");
    };

    let content_type = content_type_for_artifact(&path);
    serve_static_file(request, path, content_type)
}

fn serve_compare_export(request: Request, workspace_root: &Path, url: &str) -> Result<()> {
    let Some(path) = resolve_compare_export_request_path(workspace_root, url)? else {
        return respond_text(request, StatusCode(404), "compare export not found", "text/plain; charset=utf-8");
    };

    let content_type = content_type_for_artifact(&path);
    serve_static_file(request, path, content_type)
}

fn serve_session_export(request: Request, workspace_root: &Path, url: &str) -> Result<()> {
    let Some(path) = resolve_session_export_request_path(workspace_root, url)? else {
        return respond_text(request, StatusCode(404), "session export not found", "text/plain; charset=utf-8");
    };

    let content_type = content_type_for_artifact(&path);
    serve_static_file(request, path, content_type)
}

fn serve_asset_file(request: Request, workspace_root: &Path, url: &str) -> Result<()> {
    let Some(path) = resolve_asset_request_path(workspace_root, url)? else {
        return respond_text(request, StatusCode(404), "asset not found", "text/plain; charset=utf-8");
    };

    let content_type = content_type_for_artifact(&path);
    serve_static_file(request, path, content_type)
}

fn respond_json<T: Serialize>(request: Request, status: StatusCode, value: &T) -> Result<()> {
    let body = serde_json::to_vec_pretty(value)?;
    let response = Response::from_data(body)
        .with_status_code(status)
        .with_header(content_type_header("application/json; charset=utf-8")?);
    request.respond(response)?;
    Ok(())
}

fn respond_text(request: Request, status: StatusCode, body: &str, content_type: &str) -> Result<()> {
    let response = Response::from_string(body.to_string())
        .with_status_code(status)
        .with_header(content_type_header(content_type)?);
    request.respond(response)?;
    Ok(())
}

fn content_type_header(value: &str) -> Result<Header> {
    Header::from_bytes(&b"Content-Type"[..], value.as_bytes())
        .map_err(|_| anyhow!("failed to build content-type header"))
}

fn build_control_surface() -> GuiControlSurface {
    GuiControlSurface {
        bridge_mode: "cli_entrypoint_ready_webview_bridge_pending".to_string(),
        actions: vec![
            GuiActionDescriptor {
                action_id: "sync_state".to_string(),
                label: "Sync State".to_string(),
                description: "Refresh GUI state from backend artifacts.".to_string(),
                supports_teacher_backend: false,
                supports_session_name: false,
                supported_teacher_backends: Vec::new(),
                command_template: "cargo run -- gui-action --action sync-state".to_string(),
            },
            GuiActionDescriptor {
                action_id: "generate_curriculum".to_string(),
                label: "Generate Curriculum".to_string(),
                description: "Create a new session and generate curriculum through the selected teacher backend.".to_string(),
                supports_teacher_backend: true,
                supports_session_name: true,
                supported_teacher_backends: vec!["mock".to_string(), "local-llm".to_string()],
                command_template: "cargo run -- gui-action --action generate-curriculum --teacher-backend <mock|local-llm> --session-name \"<session-name>\"".to_string(),
            },
            GuiActionDescriptor {
                action_id: "run_session".to_string(),
                label: "Run Session".to_string(),
                description: "Create a new session, generate curriculum, execute Janet, and refresh analysis artifacts.".to_string(),
                supports_teacher_backend: true,
                supports_session_name: true,
                supported_teacher_backends: vec!["mock".to_string(), "local-llm".to_string()],
                command_template: "cargo run -- gui-action --action run-session --teacher-backend <mock|local-llm> --session-name \"<session-name>\"".to_string(),
            },
            GuiActionDescriptor {
                action_id: "update_skill_approvals".to_string(),
                label: "Confirm Skills".to_string(),
                description: "Save the selected MCM skill approvals for future runs.".to_string(),
                supports_teacher_backend: false,
                supports_session_name: false,
                supported_teacher_backends: Vec::new(),
                command_template: "cargo run -- gui-action --action update-skill-approvals".to_string(),
            },
            GuiActionDescriptor {
                action_id: "stop_run".to_string(),
                label: "Stop Run".to_string(),
                description: "Request a clean stop for the active bridge-owned run and preserve partial artifacts.".to_string(),
                supports_teacher_backend: false,
                supports_session_name: false,
                supported_teacher_backends: Vec::new(),
                command_template: "cargo run -- gui-action --action stop-run".to_string(),
            },
            GuiActionDescriptor {
                action_id: "pause_run".to_string(),
                label: "Pause Run".to_string(),
                description: "Request a cooperative pause for the active bridge-owned run." .to_string(),
                supports_teacher_backend: false,
                supports_session_name: false,
                supported_teacher_backends: Vec::new(),
                command_template: "cargo run -- gui-action --action pause-run".to_string(),
            },
            GuiActionDescriptor {
                action_id: "resume_run".to_string(),
                label: "Resume Run".to_string(),
                description: "Resume a paused bridge-owned run.".to_string(),
                supports_teacher_backend: false,
                supports_session_name: false,
                supported_teacher_backends: Vec::new(),
                command_template: "cargo run -- gui-action --action resume-run".to_string(),
            },
        ],
    }
}

fn build_setup_snapshot(config: &AppConfig, workspace_root: &Path) -> GuiSetupSnapshot {
    let runtime_path = resolve_workspace_path(workspace_root, &config.teacher.runtime.runtime_path);
    let server_binary =
        resolve_workspace_path(workspace_root, &config.teacher.runtime.server_binary);
    let model_path = resolve_workspace_path(workspace_root, &config.teacher.local_model.model_path);
    let sessions_dir = resolve_workspace_path(workspace_root, &config.session.sessions_dir);
    let aggregated_dir = resolve_workspace_path(workspace_root, &config.session.aggregated_dir);
    let endpoint_ready = probe_endpoint_ready(&config.teacher.runtime.endpoint);

    let mut warnings = Vec::new();
    if !runtime_path.exists() {
        warnings.push("Configured runtime path does not exist.".to_string());
    }
    if !server_binary.is_file() {
        warnings.push("Configured server binary is missing.".to_string());
    }
    if !model_path.is_file() {
        warnings.push("Configured local model file is missing.".to_string());
    }
    if config.teacher.backend.as_str() == "local_llm" && !endpoint_ready && !config.teacher.runtime.enabled {
        warnings.push("Local teacher endpoint is unavailable and runtime launching is disabled.".to_string());
    }
    if config.session.curriculum_size_hint.as_str() == "tiny_fixture" {
        warnings.push("Curriculum size is set to tiny_fixture, which is below smoke expectations.".to_string());
    }
    if config.session.curriculum_size_hint.as_str() == "smoke"
        && matches!(config.session.default_run_mode, crate::config::RunMode::Full)
    {
        warnings.push("Run mode is full but curriculum size hint remains smoke.".to_string());
    }

    GuiSetupSnapshot {
        environment: config.app.environment.clone(),
        configured_teacher_backend: config.teacher.backend.as_str().to_string(),
        configured_run_mode: config.session.default_run_mode.as_str().to_string(),
        curriculum_size_hint: config.session.curriculum_size_hint.as_str().to_string(),
        sessions_dir: sessions_dir.to_string_lossy().to_string(),
        aggregated_dir: aggregated_dir.to_string_lossy().to_string(),
        runtime_enabled: config.teacher.runtime.enabled,
        runtime_path: runtime_path.to_string_lossy().to_string(),
        runtime_path_exists: runtime_path.exists(),
        server_binary: server_binary.to_string_lossy().to_string(),
        server_binary_exists: server_binary.is_file(),
        endpoint: config.teacher.runtime.endpoint.clone(),
        endpoint_ready,
        model_path: model_path.to_string_lossy().to_string(),
        model_path_exists: model_path.is_file(),
        context_size: config.teacher.local_model.context_size,
        gpu_layers: config.teacher.local_model.gpu_layers,
        warnings,
    }
}

fn build_skill_snapshot(config: &AppConfig) -> GuiSkillSnapshot {
    let approved = config
        .skill_approvals
        .approved_skill_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let blocked = config
        .skill_approvals
        .blocked_skill_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();

    GuiSkillSnapshot {
        approvals_version: config.skill_approvals.approvals_version.clone(),
        manifest_version: config.skill_manifest.manifest_version.clone(),
        approved_count: approved.len(),
        blocked_count: blocked.len(),
        entries: config
            .skill_manifest
            .skills
            .iter()
            .map(|skill| GuiSkillEntry {
                skill_id: skill.skill_id.clone(),
                description: skill.description.clone(),
                deterministic: skill.deterministic,
                approved: approved.contains(&skill.skill_id),
            })
            .collect(),
    }
}

fn update_skill_approvals(config_dir: &Path, selected_skill_ids: Vec<String>) -> Result<SkillApprovals> {
    let config = AppConfig::load_from_dir(config_dir)?;
    let manifest_ids = config
        .skill_manifest
        .skills
        .iter()
        .map(|skill| skill.skill_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let selected = selected_skill_ids
        .into_iter()
        .filter(|skill_id| manifest_ids.contains(skill_id))
        .collect::<std::collections::BTreeSet<_>>();
    let blocked_skill_ids = manifest_ids
        .iter()
        .filter(|skill_id| !selected.contains(*skill_id))
        .cloned()
        .collect::<Vec<_>>();
    let approvals = SkillApprovals {
        approvals_version: config.skill_approvals.approvals_version,
        approved_skill_ids: selected.into_iter().collect(),
        blocked_skill_ids,
    };
    write_skill_approvals(config_dir, &approvals)?;
    Ok(approvals)
}

fn collect_recent_sessions(sessions_root: &Path) -> Result<Vec<GuiSessionCard>> {
    if !sessions_root.exists() {
        return Ok(Vec::new());
    }

    let mut dirs = fs::read_dir(sessions_root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect::<Vec<_>>();

    dirs.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));

    dirs.into_iter()
        .take(5)
        .map(|entry| read_session_card(&entry.path()))
        .collect()
}

fn read_session_card(session_dir: &Path) -> Result<GuiSessionCard> {
    let summary: SessionSummary =
        read_json(session_dir.join("session_summary.json")).with_context(|| {
            format!("failed to read session summary from {}", session_dir.display())
        })?;
    let run_id = summary.run_id.clone();
    let curriculum_preview = read_curriculum_preview(session_dir).ok().flatten();
    let config_snapshot: serde_json::Value =
        read_json(session_dir.join("session_config.json")).unwrap_or_else(|_| serde_json::json!({}));
    let skill_run_snapshot = read_skill_run_snapshot(&config_snapshot);
    let analysis = read_json::<AnalysisReport>(session_dir.join("analysis_report.json")).ok();
    let teacher_snapshot = read_teacher_snapshot(session_dir).ok().flatten();

    Ok(GuiSessionCard {
        session_id: summary.session_id,
        run_id: summary.run_id,
        run_mode: summary.run_mode,
        teacher_backend_id: summary.teacher_backend_id,
        completed_at: summary.completed_at,
        notes: summary.notes,
        skill_run_snapshot,
        curriculum_stats: summary.curriculum_stats,
        curriculum_preview,
        interaction_stats: summary.interaction_stats,
        memory_stats: summary.memory_stats,
        refusal_stats: summary.refusal_stats,
        anomaly_stats: summary.anomaly_stats,
        analysis_snapshot: analysis.map(|report| GuiAnalysisSnapshot {
            confirmed_count: report.confirmed_signals.len(),
            boundary_count: report.boundary_signals.len(),
            emergent_count: report.emergent_candidate_signals.len(),
            unknown_count: report.unknown_structure_candidates.len(),
            repeated_anomaly_cluster_count: report.repeated_anomaly_clusters.len(),
            category_mismatch_cluster_count: report.category_mismatch_clusters.len(),
            confirmed_summaries: report
                .confirmed_signals
                .iter()
                .take(3)
                .map(|signal| signal.summary.clone())
                .collect(),
            boundary_summaries: report
                .boundary_signals
                .iter()
                .take(3)
                .map(|signal| signal.summary.clone())
                .collect(),
            emergent_summaries: report
                .emergent_candidate_signals
                .iter()
                .take(3)
                .map(|signal| signal.summary.clone())
                .collect(),
            caution_notes: report.caution_notes,
            recommended_next_probes: report.recommended_next_probes,
        }),
        teacher_snapshot,
        telemetry_preview: read_telemetry_preview(session_dir).unwrap_or_default(),
        comparison_items: read_comparison_items(session_dir).unwrap_or_default(),
        artifacts: build_artifact_descriptors(session_dir, &run_id),
        root_dir: config_snapshot
            .get("manifest")
            .and_then(|manifest| manifest.get("root_dir"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| session_dir.to_string_lossy().to_string()),
    })
}

fn read_skill_run_snapshot(config_snapshot: &serde_json::Value) -> Option<GuiSessionSkillRunSnapshot> {
    let manifest_skills = config_snapshot
        .get("skill_manifest")
        .and_then(|manifest| manifest.get("skills"))
        .and_then(|skills| skills.as_array())?;
    let total_skill_count = manifest_skills.len();
    if total_skill_count == 0 {
        return None;
    }

    let approved_skill_ids = config_snapshot
        .get("skill_approvals")
        .and_then(|approvals| approvals.get("approved_skill_ids"))
        .and_then(|skills| skills.as_array())
        .map(|skills| {
            skills
                .iter()
                .filter_map(|skill| skill.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let blocked_skill_ids = config_snapshot
        .get("skill_approvals")
        .and_then(|approvals| approvals.get("blocked_skill_ids"))
        .and_then(|skills| skills.as_array())
        .map(|skills| {
            skills
                .iter()
                .filter_map(|skill| skill.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let approved_count = approved_skill_ids.len();
    let selection_mode = if approved_count == 0 {
        "memory_only".to_string()
    } else if approved_count == total_skill_count {
        "all_skills".to_string()
    } else {
        "restricted".to_string()
    };

    Some(GuiSessionSkillRunSnapshot {
        selection_mode,
        approved_count,
        total_skill_count,
        approved_skill_ids,
        blocked_skill_ids,
    })
}

fn read_teacher_snapshot(session_dir: &Path) -> Result<Option<GuiTeacherSnapshot>> {
    let path = session_dir.join("teacher_calls.jsonl");
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read teacher calls from {}", path.display()))?;
    let Some(last_line) = raw.lines().rev().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };

    let record: TeacherCallRecord = serde_json::from_str(last_line)
        .with_context(|| format!("failed to parse teacher call from {}", path.display()))?;

    let response_outline = record.response_payload.get("outline");
    let rationale = response_outline
        .and_then(|outline| outline.get("rationale"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string);
    let selected_domain_ids = response_outline
        .and_then(|outline| outline.get("domains"))
        .and_then(|value| value.as_array())
        .map(|domains| {
            domains
                .iter()
                .filter_map(|domain| domain.get("domain_id").and_then(|value| value.as_str()))
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_concept_ids = response_outline
        .and_then(|outline| outline.get("domains"))
        .and_then(|value| value.as_array())
        .map(|domains| {
            domains
                .iter()
                .flat_map(|domain| {
                    domain
                        .get("concepts")
                        .and_then(|value| value.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|concept| concept.get("concept_id").and_then(|value| value.as_str()))
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(Some(GuiTeacherSnapshot {
        teacher_backend_id: record.teacher_backend_id,
        operation: record.operation,
        timestamp: record.timestamp,
        success: record.success,
        latency_ms: record.latency_ms,
        token_counts: record.token_counts,
        launched_runtime: record
            .runtime_config
            .get("launched_runtime")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        endpoint_ready: record
            .runtime_config
            .get("endpoint_ready")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        model_kind: record
            .model_config
            .get("kind")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        model_path: record
            .model_config
            .get("model_path")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        rationale,
        selected_domain_ids,
        selected_concept_ids,
        error: record.error,
    }))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read json file {}", path.display()))?;
    Ok(serde_json::from_str(&raw)?)
}

fn read_curriculum_preview(session_dir: &Path) -> Result<Option<GuiCurriculumPreview>> {
    let generated_path = session_dir.join("curriculum_generated.jsonl");
    if !generated_path.exists() {
        return Ok(None);
    }

    let curriculum = read_jsonl::<CurriculumBundle>(generated_path)?
        .into_iter()
        .next();
    let Some(curriculum) = curriculum else {
        return Ok(None);
    };

    let validation = read_jsonl::<CurriculumValidationReport>(
        session_dir.join("curriculum_validated.jsonl"),
    )?
    .into_iter()
    .next();

    let warnings = build_curriculum_warnings(&curriculum, validation.as_ref());
    let item_mix = summarize_item_mix(&curriculum);
    let domain_summaries = curriculum
        .domains
        .iter()
        .map(|domain| {
            let domain_items = curriculum
                .items
                .iter()
                .filter(|item| item.domain_id == domain.domain_id)
                .collect::<Vec<_>>();
            GuiCurriculumDomainSummary {
                domain_id: domain.domain_id.clone(),
                name: domain.name.clone(),
                concept_count: domain.concepts.len(),
                item_count: domain_items.len(),
                probe_count: domain_items
                    .iter()
                    .filter(|item| item.item_type.starts_with("probe_"))
                    .count(),
                concepts: domain
                    .concepts
                    .iter()
                    .map(|concept| concept.name.clone())
                    .collect(),
            }
        })
        .collect();
    let sample_items = curriculum
        .items
        .iter()
        .take(6)
        .map(|item| GuiCurriculumItemPreview {
            item_id: item.item_id.clone(),
            domain_id: item.domain_id.clone(),
            concept_id: item.concept_id.clone(),
            item_type: item.item_type.clone(),
            prompt: item.prompt.clone(),
            expected_answer: item.expected_answer.clone(),
            expected_skills: item.expected_skills.clone(),
            novelty_class: item.novelty_class.clone(),
            probe_role: item.probe_role.clone(),
            boundary_kind: item.boundary_kind.clone(),
        })
        .collect();

    Ok(Some(GuiCurriculumPreview {
        generation_notes: curriculum.generation_notes,
        warnings,
        domain_summaries,
        item_mix,
        sample_items,
    }))
}

fn summarize_item_mix(curriculum: &CurriculumBundle) -> Vec<GuiCountMetric> {
    [
        ("Teaching", "teaching"),
        ("Near-transfer probes", "probe_near_transfer"),
        ("Boundary probes", "probe_boundary"),
    ]
    .into_iter()
    .map(|(label, kind)| GuiCountMetric {
        label: label.to_string(),
        count: curriculum
            .items
            .iter()
            .filter(|item| item.item_type == kind)
            .count(),
    })
    .collect()
}

fn build_curriculum_warnings(
    curriculum: &CurriculumBundle,
    validation: Option<&CurriculumValidationReport>,
) -> Vec<String> {
    let mut warnings = validation
        .map(|report| report.warnings.clone())
        .unwrap_or_default();
    let summary = curriculum.summary();

    if summary.item_count < 20 {
        warnings.push("Curriculum is smaller than the expected smoke-session floor.".to_string());
    }
    if summary.probe_count < summary.concept_count / 2 {
        warnings.push("Probe coverage looks thin relative to concept count.".to_string());
    }
    if curriculum
        .items
        .iter()
        .any(|item| item.prompt.trim().is_empty() || item.expected_answer.is_none())
    {
        warnings.push(
            "One or more curriculum items are missing required prompt or answer fields."
                .to_string(),
        );
    }

    warnings
}

fn build_artifact_descriptors(session_dir: &Path, run_id: &str) -> Vec<GuiArtifactDescriptor> {
    [
        ("Analysis Report (Markdown)", "analysis_report.md", "text/markdown; charset=utf-8"),
        ("Analysis Report (JSON)", "analysis_report.json", "application/json; charset=utf-8"),
        ("Session Summary", "session_summary.json", "application/json; charset=utf-8"),
        ("Curriculum Generated", "curriculum_generated.jsonl", "application/x-ndjson; charset=utf-8"),
        ("Teacher Calls", "teacher_calls.jsonl", "application/x-ndjson; charset=utf-8"),
        ("Telemetry", "telemetry.jsonl", "application/x-ndjson; charset=utf-8"),
        ("Refusal Events", "refusal_events.jsonl", "application/x-ndjson; charset=utf-8"),
        ("Anomaly Events", "anomaly_events.jsonl", "application/x-ndjson; charset=utf-8"),
    ]
    .into_iter()
    .filter_map(|(label, file_name, content_type)| {
        let absolute = session_dir.join(file_name);
        absolute.exists().then(|| GuiArtifactDescriptor {
            label: label.to_string(),
            file_name: file_name.to_string(),
            absolute_path: absolute.to_string_lossy().to_string(),
            relative_path: file_name.to_string(),
            content_type: content_type.to_string(),
            download_path: format!("/artifacts/{run_id}/{file_name}"),
        })
    })
    .collect()
}

fn read_telemetry_preview(session_dir: &Path) -> Result<Vec<GuiTelemetryPreviewRow>> {
    let interactions =
        read_jsonl::<InteractionEvent>(session_dir.join("interactions.jsonl")).unwrap_or_default();
    let traces = read_jsonl::<McmTraceEvent>(session_dir.join("mcm_trace.jsonl")).unwrap_or_default();
    let skill_events =
        read_jsonl::<SkillEvent>(session_dir.join("skill_events.jsonl")).unwrap_or_default();
    let memory_events =
        read_jsonl::<MemoryEvent>(session_dir.join("memory_events.jsonl")).unwrap_or_default();
    let refusal_events =
        read_jsonl::<RefusalEvent>(session_dir.join("refusal_events.jsonl")).unwrap_or_default();
    let anomaly_events =
        read_jsonl::<AnomalyEvent>(session_dir.join("anomaly_events.jsonl")).unwrap_or_default();

    let trace_by_item = traces
        .into_iter()
        .map(|trace| (trace.item_id.clone(), trace))
        .collect::<std::collections::HashMap<_, _>>();
    let skill_by_item = skill_events
        .into_iter()
        .map(|event| (event.item_id.clone(), event))
        .collect::<std::collections::HashMap<_, _>>();
    let refusal_by_item = refusal_events
        .into_iter()
        .map(|event| (event.item_id.clone(), event))
        .collect::<std::collections::HashMap<_, _>>();
    let anomaly_by_item = anomaly_events
        .into_iter()
        .map(|event| (event.item_id.clone(), event))
        .collect::<std::collections::HashMap<_, _>>();
    let mut memory_reads_by_item = std::collections::HashMap::<String, Vec<String>>::new();
    for event in memory_events {
        if event.operation == "read"
            && let Some(item_id) = event.source_item_id
        {
            memory_reads_by_item
                .entry(item_id)
                .or_default()
                .push(event.key);
        }
    }

    Ok(interactions
        .into_iter()
        .rev()
        .take(6)
        .map(|interaction| {
            let trace = trace_by_item.get(&interaction.item_id);
            let skill = skill_by_item.get(&interaction.item_id);
            let refusal = refusal_by_item.get(&interaction.item_id);
            let anomaly = anomaly_by_item.get(&interaction.item_id);

            GuiTelemetryPreviewRow {
                item_id: interaction.item_id.clone(),
                item_type: interaction.item_type.clone(),
                prompt: interaction.generated_question,
                expected_answer: interaction.expected_answer,
                janet_answer: interaction.janet_answer,
                correctness_judgment: interaction.correctness_judgment,
                structure_fit: interaction.structure_fit,
                anomaly_flags: interaction.anomaly_flags,
                final_mode: trace.map(|entry| entry.final_mode.clone()),
                uncertainty_state: trace.map(|entry| entry.uncertainty_state.clone()),
                executed_skill: trace
                    .and_then(|entry| entry.executed_skill.clone())
                    .or_else(|| skill.and_then(|entry| entry.executed_skill.clone())),
                candidate_skills: trace
                    .map(|entry| entry.candidate_skills.clone())
                    .or_else(|| skill.map(|entry| entry.candidate_skills.clone()))
                    .unwrap_or_default(),
                approved_skills: trace
                    .map(|entry| entry.approved_skills.clone())
                    .or_else(|| skill.map(|entry| entry.approved_skills.clone()))
                    .unwrap_or_default(),
                blocked_skills: trace
                    .map(|entry| entry.blocked_skills.clone())
                    .or_else(|| skill.map(|entry| entry.blocked_skills.clone()))
                    .unwrap_or_default(),
                policy_checks: trace
                    .map(|entry| entry.policy_checks.clone())
                    .unwrap_or_default(),
                reasoning_steps: trace
                    .map(|entry| entry.reasoning_steps.clone())
                    .unwrap_or_default(),
                memory_reads: memory_reads_by_item
                    .get(&interaction.item_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        trace
                            .map(|entry| entry.memory_reads.clone())
                            .unwrap_or_default()
                    }),
                refusal_reason: refusal
                    .map(|entry| entry.reason.clone())
                    .or_else(|| trace.and_then(|entry| entry.refusal_reason.clone())),
                refusal_next_steps: refusal
                    .map(|entry| entry.candidate_next_steps.clone())
                    .unwrap_or_default(),
                anomaly_explanation: anomaly.map(|entry| entry.structure_fit_explanation.clone()),
            }
        })
        .collect())
}

fn compare_exports_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("compare_exports")
}

fn session_exports_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("session_exports")
}

fn save_compare_export(
    workspace_root: &Path,
    payload: GuiCompareExportRequest,
) -> Result<GuiCompareExportSaved> {
    let file_name = sanitize_compare_export_file_name(&payload.file_name)?;
    let exports_dir = compare_exports_dir(workspace_root);
    fs::create_dir_all(&exports_dir)
        .with_context(|| format!("failed to create compare export dir {}", exports_dir.display()))?;

    let export_path = exports_dir.join(&file_name);
    fs::write(&export_path, payload.content.as_bytes())
        .with_context(|| format!("failed to write compare export {}", export_path.display()))?;

    Ok(GuiCompareExportSaved {
        file_name: file_name.clone(),
        absolute_path: export_path.to_string_lossy().to_string(),
        relative_path: format!("compare_exports/{}", file_name),
        download_path: format!("/compare-exports/{}", file_name),
        content_type: payload.content_type,
    })
}

fn save_session_export_bundle(
    workspace_root: &Path,
    config: &AppConfig,
    payload: GuiSessionExportRequest,
) -> Result<GuiCompareExportSaved> {
    let session_dir = find_session_dir_for_run_id(config, &payload.run_id)?
        .ok_or_else(|| anyhow!("session {} not found", payload.run_id))?;
    let exports_dir = session_exports_dir(workspace_root);
    fs::create_dir_all(&exports_dir)
        .with_context(|| format!("failed to create session export dir {}", exports_dir.display()))?;

    let file_name = format!("janet-session-{}-bundle.json", &payload.run_id[..8]);
    let export_path = exports_dir.join(&file_name);
    let bundle = build_session_export_bundle(&session_dir)?;
    fs::write(&export_path, serde_json::to_vec_pretty(&bundle)?)
        .with_context(|| format!("failed to write session export {}", export_path.display()))?;

    Ok(GuiCompareExportSaved {
        file_name: file_name.clone(),
        absolute_path: export_path.to_string_lossy().to_string(),
        relative_path: format!("session_exports/{}", file_name),
        download_path: format!("/session-exports/{}", file_name),
        content_type: "application/json; charset=utf-8".to_string(),
    })
}

fn build_session_export_bundle(session_dir: &Path) -> Result<serde_json::Value> {
    let session_summary: serde_json::Value = read_json(session_dir.join("session_summary.json"))?;
    let session_config: serde_json::Value =
        read_json(session_dir.join("session_config.json")).unwrap_or_else(|_| serde_json::json!({}));
    let analysis_report: serde_json::Value =
        read_json(session_dir.join("analysis_report.json")).unwrap_or_else(|_| serde_json::json!({}));
    let analysis_markdown =
        fs::read_to_string(session_dir.join("analysis_report.md")).unwrap_or_default();
    let teacher_calls = read_jsonl::<serde_json::Value>(session_dir.join("teacher_calls.jsonl"))?;
    let interactions = read_jsonl::<serde_json::Value>(session_dir.join("interactions.jsonl"))?;
    let telemetry = read_jsonl::<serde_json::Value>(session_dir.join("telemetry.jsonl"))?;
    let refusals = read_jsonl::<serde_json::Value>(session_dir.join("refusal_events.jsonl"))?;
    let anomalies = read_jsonl::<serde_json::Value>(session_dir.join("anomaly_events.jsonl"))?;
    let memory_events = read_jsonl::<serde_json::Value>(session_dir.join("memory_events.jsonl"))?;
    let skill_events = read_jsonl::<serde_json::Value>(session_dir.join("skill_events.jsonl"))?;
    let transfer_probes = read_jsonl::<serde_json::Value>(session_dir.join("transfer_probes.jsonl"))?;
    let mcm_trace = read_jsonl::<serde_json::Value>(session_dir.join("mcm_trace.jsonl"))?;
    let curriculum_generated =
        read_jsonl::<serde_json::Value>(session_dir.join("curriculum_generated.jsonl"))?;
    let curriculum_validated =
        read_jsonl::<serde_json::Value>(session_dir.join("curriculum_validated.jsonl"))?;

    Ok(serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "session_root": session_dir.to_string_lossy().to_string(),
        "session_summary": session_summary,
        "session_config": session_config,
        "analysis_report": analysis_report,
        "analysis_report_markdown": analysis_markdown,
        "curriculum_generated": curriculum_generated,
        "curriculum_validated": curriculum_validated,
        "teacher_calls": teacher_calls,
        "interactions": interactions,
        "mcm_trace": mcm_trace,
        "telemetry": telemetry,
        "memory_events": memory_events,
        "skill_events": skill_events,
        "refusal_events": refusals,
        "transfer_probes": transfer_probes,
        "anomaly_events": anomalies,
    }))
}

fn open_session_folder(config: &AppConfig, payload: GuiSessionExportRequest) -> Result<GuiOpenFolderResult> {
    let session_dir = find_session_dir_for_run_id(config, &payload.run_id)?
        .ok_or_else(|| anyhow!("session {} not found", payload.run_id))?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&session_dir)
            .spawn()
            .with_context(|| format!("failed to open session folder {}", session_dir.display()))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        bail!("open session folder is currently only implemented for Windows hosts");
    }

    Ok(GuiOpenFolderResult {
        opened: true,
        absolute_path: session_dir.to_string_lossy().to_string(),
    })
}

fn sanitize_compare_export_file_name(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("compare export file name cannot be empty");
    }
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        bail!("invalid compare export file name");
    }

    let extension = Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if !matches!(extension, "md" | "json") {
        bail!("unsupported compare export extension: {}", extension);
    }

    Ok(value.to_string())
}

fn read_comparison_items(session_dir: &Path) -> Result<Vec<GuiComparisonItemOutcome>> {
    let interactions =
        read_jsonl::<InteractionEvent>(session_dir.join("interactions.jsonl")).unwrap_or_default();
    let traces = read_jsonl::<McmTraceEvent>(session_dir.join("mcm_trace.jsonl")).unwrap_or_default();
    let refusal_events =
        read_jsonl::<RefusalEvent>(session_dir.join("refusal_events.jsonl")).unwrap_or_default();
    let anomaly_events =
        read_jsonl::<AnomalyEvent>(session_dir.join("anomaly_events.jsonl")).unwrap_or_default();

    let trace_by_item = traces
        .into_iter()
        .map(|trace| (trace.item_id.clone(), trace))
        .collect::<std::collections::HashMap<_, _>>();
    let refusal_by_item = refusal_events
        .into_iter()
        .map(|event| (event.item_id.clone(), event))
        .collect::<std::collections::HashMap<_, _>>();
    let anomaly_by_item = anomaly_events
        .into_iter()
        .map(|event| (event.item_id.clone(), event))
        .collect::<std::collections::HashMap<_, _>>();

    Ok(interactions
        .into_iter()
        .map(|interaction| {
            let trace = trace_by_item.get(&interaction.item_id);
            let refusal = refusal_by_item.get(&interaction.item_id);
            let anomaly = anomaly_by_item.get(&interaction.item_id);

            GuiComparisonItemOutcome {
                item_id: interaction.item_id,
                item_type: interaction.item_type,
                prompt: interaction.generated_question,
                expected_answer: interaction.expected_answer,
                janet_answer: interaction.janet_answer,
                correctness_judgment: interaction.correctness_judgment,
                structure_fit: interaction.structure_fit,
                executed_skill: trace.and_then(|entry| entry.executed_skill.clone()),
                refusal_reason: refusal
                    .map(|entry| entry.reason.clone())
                    .or_else(|| trace.and_then(|entry| entry.refusal_reason.clone())),
                anomaly_flags: anomaly
                    .map(|entry| entry.anomaly_flags.clone())
                    .unwrap_or(interaction.anomaly_flags),
            }
        })
        .collect())
}

fn resolve_artifact_request_path(config: &AppConfig, url: &str) -> Result<Option<PathBuf>> {
    let segments = url
        .trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or_default()
        .split('/')
        .collect::<Vec<_>>();

    if segments.len() != 3 || segments[0] != "artifacts" {
        return Ok(None);
    }

    let run_id = segments[1];
    let file_name = segments[2];
    if file_name.contains('\\') || file_name.contains('/') || file_name.contains("..") {
        bail!("invalid artifact path");
    }

    let allowed = [
        "analysis_report.md",
        "analysis_report.json",
        "session_summary.json",
        "curriculum_generated.jsonl",
        "teacher_calls.jsonl",
        "telemetry.jsonl",
        "refusal_events.jsonl",
        "anomaly_events.jsonl",
    ];
    if !allowed.contains(&file_name) {
        bail!("unsupported artifact {}", file_name);
    }

    if let Some(path) = find_session_dir_for_run_id(config, run_id)? {
        let artifact_path = path.join(file_name);
        return Ok(artifact_path.exists().then_some(artifact_path));
    }

    Ok(None)
}

fn resolve_compare_export_request_path(workspace_root: &Path, url: &str) -> Result<Option<PathBuf>> {
    let segments = url
        .trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or_default()
        .split('/')
        .collect::<Vec<_>>();

    if segments.len() != 2 || segments[0] != "compare-exports" {
        return Ok(None);
    }

    let file_name = sanitize_compare_export_file_name(segments[1])?;
    let export_path = compare_exports_dir(workspace_root).join(file_name);
    Ok(export_path.exists().then_some(export_path))
}

fn resolve_session_export_request_path(workspace_root: &Path, url: &str) -> Result<Option<PathBuf>> {
    let segments = url
        .trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or_default()
        .split('/')
        .collect::<Vec<_>>();

    if segments.len() != 2 || segments[0] != "session-exports" {
        return Ok(None);
    }

    let file_name = sanitize_session_export_file_name(segments[1])?;
    let export_path = session_exports_dir(workspace_root).join(file_name);
    Ok(export_path.exists().then_some(export_path))
}

fn find_session_dir_for_run_id(config: &AppConfig, run_id: &str) -> Result<Option<PathBuf>> {
    let sessions_root = PathBuf::from(&config.session.sessions_dir);
    if !sessions_root.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(&sessions_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let summary_path = path.join("session_summary.json");
        if !summary_path.exists() {
            continue;
        }
        let summary: SessionSummary = read_json(summary_path)?;
        if summary.run_id == run_id {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn sanitize_session_export_file_name(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("session export file name cannot be empty");
    }
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        bail!("invalid session export file name");
    }

    let extension = Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if extension != "json" {
        bail!("unsupported session export extension: {}", extension);
    }

    Ok(value.to_string())
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read jsonl file {}", path.display()))?;
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<Vec<T>, _>>()
        .map_err(Into::into)
}

fn content_type_for_artifact(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "md" => "text/markdown; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "jsonl" => "application/x-ndjson; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn resolve_asset_request_path(workspace_root: &Path, url: &str) -> Result<Option<PathBuf>> {
    let Some(relative) = url.strip_prefix("/assets/") else {
        return Ok(None);
    };
    if relative.is_empty() || relative.contains("..") || relative.contains('\\') {
        return Ok(None);
    }

    let path = workspace_root.join("assets").join(relative);
    if path.is_file() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn resolve_workspace_path(workspace_root: &Path, value: &str) -> PathBuf {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace_root.join(candidate)
    }
}

fn probe_endpoint_ready(endpoint: &str) -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    let models_url = format!("{}/models", endpoint.trim_end_matches('/'));
    client
        .get(models_url)
        .send()
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    use crate::config::{
        AppMetadata, CurriculumSizeHint, McmConfig, RunMode, RuntimeConfig, SessionConfig,
        SkillApprovals, SkillManifest, SkillManifestEntry, TeacherConfig,
    };

    #[test]
    fn bridge_status_prefers_active_running_job_and_limits_recent_history() {
        let runtime_state = Arc::new(Mutex::new(BridgeRuntimeState {
            jobs: vec![
                job("job-running", "running"),
                job("job-queued", "queued"),
                job("job-completed-1", "completed"),
                job("job-completed-2", "completed"),
                job("job-completed-3", "completed"),
                job("job-completed-4", "completed"),
            ],
        }));

        let status = build_bridge_status(&runtime_state).expect("bridge status should build");

        assert_eq!(status.active_job.as_ref().map(|job| job.job_id.as_str()), Some("job-running"));
        assert_eq!(status.recent_jobs.len(), 5);
        assert_eq!(status.recent_jobs[0].job_id, "job-running");
        assert_eq!(status.recent_jobs[4].job_id, "job-completed-3");
    }

    #[test]
    fn progress_aware_run_session_updates_gui_state_and_emits_completion() {
        let root = unique_test_root("gui-progress-run");
        let config_dir = root.join("config");
        let workspace_dir = root.join("workspace");
        fs::create_dir_all(&config_dir).expect("config dir should exist");
        fs::create_dir_all(workspace_dir.join("web")).expect("web dir should exist");

        write_test_config(&config_dir, &workspace_dir);

        let mut progress_updates = Vec::new();
        let value = execute_gui_action_request_with_progress(
            &config_dir,
            &workspace_dir,
            GuiActionRequest {
                action: "run_session".to_string(),
                session_name: Some("GUI Progress Smoke".to_string()),
                teacher_backend: Some("mock".to_string()),
                selected_skill_ids: None,
                run_mode: None,
                curriculum_size_hint: None,
                model_path: None,
                endpoint: None,
                sessions_dir: None,
                aggregated_dir: None,
            },
            || RunControlSignal::Continue,
            |progress| progress_updates.push(progress),
        )
        .expect("mock gui run should succeed");

        let run_stats = value
            .get("run_stats")
            .expect("run stats should be present in completed response");
        assert_eq!(run_stats.get("total_items").and_then(|value| value.as_u64()), Some(30));
        assert!(progress_updates.iter().any(|progress| progress.phase == "session_initialized"));
        assert!(progress_updates.iter().any(|progress| progress.phase == "curriculum_generated"));
        assert!(progress_updates.iter().any(|progress| progress.phase == "session_completed"));

        let gui_state_path = workspace_dir.join("web").join("gui-state.json");
        assert!(gui_state_path.is_file());
        let gui_state: GuiState = serde_json::from_slice(
            &fs::read(&gui_state_path).expect("gui state file should be readable"),
        )
        .expect("gui state json should parse");
        assert_eq!(gui_state.control_surface.bridge_mode, "cli_entrypoint_ready_webview_bridge_pending");
        assert!(gui_state.latest_session.is_some());

        fs::remove_dir_all(&root).expect("test workspace should clean up");
    }

    #[test]
    fn save_setup_persists_config_updates() {
        let root = unique_test_root("gui-save-setup");
        let config_dir = root.join("config");
        let workspace_dir = root.join("workspace");
        fs::create_dir_all(&config_dir).expect("config dir should exist");
        fs::create_dir_all(workspace_dir.join("web")).expect("web dir should exist");

        write_test_config(&config_dir, &workspace_dir);

        let value = execute_gui_action_request(
            &config_dir,
            &workspace_dir,
            GuiActionRequest {
                action: "save_setup".to_string(),
                session_name: None,
                teacher_backend: Some("local-llm".to_string()),
                selected_skill_ids: None,
                run_mode: Some("full".to_string()),
                curriculum_size_hint: Some("full".to_string()),
                model_path: Some("models/new-teacher.gguf".to_string()),
                endpoint: Some("http://127.0.0.1:9000/v1".to_string()),
                sessions_dir: Some("data/custom_sessions".to_string()),
                aggregated_dir: Some("data/custom_aggregated".to_string()),
            },
        )
        .expect("save setup should succeed");

        let state: GuiState = serde_json::from_value(value).expect("gui state should deserialize");
        assert_eq!(state.configured_teacher_backend, "local_llm");
        assert_eq!(state.setup_snapshot.configured_run_mode, "full");
        assert_eq!(state.setup_snapshot.curriculum_size_hint, "full");
        assert_eq!(state.setup_snapshot.endpoint, "http://127.0.0.1:9000/v1");

        let reloaded = AppConfig::load_from_dir(&config_dir).expect("config should reload");
        assert_eq!(reloaded.teacher.backend.as_str(), "local_llm");
        assert_eq!(reloaded.session.default_run_mode.as_str(), "full");
        assert_eq!(reloaded.session.curriculum_size_hint.as_str(), "full");
        assert_eq!(reloaded.teacher.local_model.model_path, "models/new-teacher.gguf");
        assert_eq!(reloaded.teacher.runtime.endpoint, "http://127.0.0.1:9000/v1");
        assert_eq!(reloaded.session.sessions_dir, "data/custom_sessions");
        assert_eq!(reloaded.session.aggregated_dir, "data/custom_aggregated");

        fs::remove_dir_all(&root).expect("test workspace should clean up");
    }

    #[test]
    fn session_bundle_export_writes_workspace_json_bundle() {
        let root = unique_test_root("gui-session-export");
        let config_dir = root.join("config");
        let workspace_dir = root.join("workspace");
        fs::create_dir_all(&config_dir).expect("config dir should exist");
        fs::create_dir_all(workspace_dir.join("web")).expect("web dir should exist");

        write_test_config(&config_dir, &workspace_dir);

        let completed = execute_gui_action_request(
            &config_dir,
            &workspace_dir,
            GuiActionRequest {
                action: "run_session".to_string(),
                session_name: Some("Export Bundle Smoke".to_string()),
                teacher_backend: Some("mock".to_string()),
                selected_skill_ids: None,
                run_mode: None,
                curriculum_size_hint: None,
                model_path: None,
                endpoint: None,
                sessions_dir: None,
                aggregated_dir: None,
            },
        )
        .expect("run session should succeed");

        let run_id = completed
            .get("generated")
            .and_then(|generated| generated.get("created"))
            .and_then(|created| created.get("manifest"))
            .and_then(|manifest| manifest.get("run_id"))
            .and_then(|value| value.as_str())
            .expect("run id should be present")
            .to_string();

        let config = AppConfig::load_from_dir(&config_dir).expect("config should reload");
        let saved = save_session_export_bundle(
            &workspace_dir,
            &config,
            GuiSessionExportRequest { run_id },
        )
        .expect("session bundle export should succeed");

        assert_eq!(saved.relative_path, format!("session_exports/{}", saved.file_name));
        let saved_path = workspace_dir.join(&saved.relative_path);
        assert!(saved_path.is_file());

        let bundle: serde_json::Value =
            serde_json::from_slice(&fs::read(saved_path).expect("bundle file should read"))
                .expect("bundle should parse");
        assert!(bundle.get("session_summary").is_some());
        assert!(bundle.get("analysis_report").is_some());
        assert!(bundle.get("interactions").and_then(|value| value.as_array()).is_some());
        assert!(bundle.get("telemetry").and_then(|value| value.as_array()).is_some());

        fs::remove_dir_all(&root).expect("test workspace should clean up");
    }

    fn job(job_id: &str, state: &str) -> GuiActionJob {
        GuiActionJob {
            job_id: job_id.to_string(),
            action: "run_session".to_string(),
            teacher_backend: Some("mock".to_string()),
            session_name: Some("Test".to_string()),
            state: state.to_string(),
            cancel_requested: false,
            pause_requested: false,
            created_at: Utc::now().to_rfc3339(),
            started_at: None,
            finished_at: None,
            result_summary: None,
            progress: None,
            error: None,
        }
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("janet-school-{label}-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root should be created");
        root
    }

    fn write_test_config(config_dir: &Path, workspace_dir: &Path) {
        let data_dir = workspace_dir.join("data");
        let sessions_dir = data_dir.join("sessions");
        let aggregated_dir = data_dir.join("aggregated");
        fs::create_dir_all(&sessions_dir).expect("sessions dir should exist");
        fs::create_dir_all(&aggregated_dir).expect("aggregated dir should exist");

        let session = SessionConfig {
            default_run_mode: RunMode::Smoke,
            sessions_dir: sessions_dir.to_string_lossy().to_string(),
            aggregated_dir: aggregated_dir.to_string_lossy().to_string(),
            curriculum_size_hint: CurriculumSizeHint::Smoke,
        };
        let app = AppMetadata {
            app_name: "Janet School".to_string(),
            version: "0.1.0".to_string(),
            environment: "test".to_string(),
            docs_dir: "docs".to_string(),
            data_dir: "data".to_string(),
            web_dir: "web".to_string(),
            show_splash: true,
            splash_duration_ms: 3000,
            session: session.clone(),
        };
        let teacher = TeacherConfig {
            backend: TeacherBackendKind::Mock,
            runtime: RuntimeConfig {
                enabled: false,
                runtime_path: "runtime".to_string(),
                server_binary: "runtime/llama-server.exe".to_string(),
                endpoint: "http://127.0.0.1:8080/v1".to_string(),
            },
            local_model: crate::config::LocalModelConfig {
                model_path: "models/placeholder.gguf".to_string(),
                context_size: 4096,
                gpu_layers: 0,
            },
        };
        let mcm = McmConfig {
            class_label: "janet".to_string(),
            deterministic_only: true,
            refusal_mode: "strict".to_string(),
            memory_store: "explicit_exact_answers".to_string(),
            policy_version: "test".to_string(),
        };
        let skill_manifest = SkillManifest {
            manifest_version: "test".to_string(),
            skills: [
                "option_match_selector",
                "ordered_relation_compare",
                "same_different_compare",
                "first_last_selector",
                "more_less_compare",
                "equal_compare",
                "left_right_selector",
                "inside_outside_selector",
                "exact_match_lookup",
            ]
            .into_iter()
            .map(|skill_id| SkillManifestEntry {
                skill_id: skill_id.to_string(),
                description: format!("{skill_id} for tests"),
                deterministic: true,
            })
            .collect(),
        };
        let approvals = SkillApprovals {
            approvals_version: "test".to_string(),
            approved_skill_ids: skill_manifest
                .skills
                .iter()
                .map(|entry| entry.skill_id.clone())
                .collect(),
            blocked_skill_ids: Vec::new(),
        };

        fs::write(
            config_dir.join("app_config.json"),
            serde_json::to_vec_pretty(&app).expect("app config should serialize"),
        )
        .expect("app config should write");
        fs::write(
            config_dir.join("teacher_config.json"),
            serde_json::to_vec_pretty(&teacher).expect("teacher config should serialize"),
        )
        .expect("teacher config should write");
        fs::write(
            config_dir.join("mcm_config.json"),
            serde_json::to_vec_pretty(&mcm).expect("mcm config should serialize"),
        )
        .expect("mcm config should write");
        fs::write(
            config_dir.join("skill_manifest.json"),
            serde_json::to_vec_pretty(&skill_manifest).expect("skill manifest should serialize"),
        )
        .expect("skill manifest should write");
        fs::write(
            config_dir.join("skill_approvals.json"),
            serde_json::to_vec_pretty(&approvals).expect("skill approvals should serialize"),
        )
        .expect("skill approvals should write");
    }
}

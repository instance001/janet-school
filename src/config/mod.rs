use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AppConfig {
    pub app: AppMetadata,
    pub session: SessionConfig,
    pub teacher: TeacherConfig,
    pub mcm: McmConfig,
    pub skill_manifest: SkillManifest,
    pub skill_approvals: SkillApprovals,
}

impl AppConfig {
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let app: AppMetadata = read_json(dir.join("app_config.json"))?;
        let session: SessionConfig = app.session.clone();
        let teacher: TeacherConfig = read_json(dir.join("teacher_config.json"))?;
        let mcm: McmConfig = read_json(dir.join("mcm_config.json"))?;
        let skill_manifest: SkillManifest = read_json(dir.join("skill_manifest.json"))?;
        let skill_approvals: SkillApprovals = read_json(dir.join("skill_approvals.json"))?;

        Ok(Self {
            app,
            session,
            teacher,
            mcm,
            skill_manifest,
            skill_approvals,
        })
    }

    pub fn resolved_against(mut self, root: &Path) -> Self {
        self.app.docs_dir = resolve_path_string(root, &self.app.docs_dir);
        self.app.data_dir = resolve_path_string(root, &self.app.data_dir);
        self.app.web_dir = resolve_path_string(root, &self.app.web_dir);
        self.session.sessions_dir = resolve_path_string(root, &self.session.sessions_dir);
        self.session.aggregated_dir = resolve_path_string(root, &self.session.aggregated_dir);
        self.app.session = self.session.clone();
        self.teacher.runtime.runtime_path =
            resolve_path_string(root, &self.teacher.runtime.runtime_path);
        self.teacher.runtime.server_binary =
            resolve_path_string(root, &self.teacher.runtime.server_binary);
        self.teacher.local_model.model_path =
            resolve_path_string(root, &self.teacher.local_model.model_path);
        self
    }
}

fn resolve_path_string(root: &Path, value: &str) -> String {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
    .display()
    .to_string()
}

pub fn write_skill_approvals(dir: &Path, approvals: &SkillApprovals) -> Result<()> {
    let path = dir.join("skill_approvals.json");
    fs::write(&path, serde_json::to_vec_pretty(approvals)?)
        .with_context(|| format!("failed to write config file {}", path.display()))
}

pub fn write_app_metadata(dir: &Path, app: &AppMetadata) -> Result<()> {
    let path = dir.join("app_config.json");
    fs::write(&path, serde_json::to_vec_pretty(app)?)
        .with_context(|| format!("failed to write config file {}", path.display()))
}

pub fn write_teacher_config(dir: &Path, teacher: &TeacherConfig) -> Result<()> {
    let path = dir.join("teacher_config.json");
    fs::write(&path, serde_json::to_vec_pretty(teacher)?)
        .with_context(|| format!("failed to write config file {}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse config file {}", path.display()))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AppMetadata {
    pub app_name: String,
    pub version: String,
    pub environment: String,
    pub docs_dir: String,
    pub data_dir: String,
    pub web_dir: String,
    pub show_splash: bool,
    pub splash_duration_ms: u64,
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionConfig {
    pub default_run_mode: RunMode,
    pub sessions_dir: String,
    pub aggregated_dir: String,
    pub curriculum_size_hint: CurriculumSizeHint,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Smoke,
    Full,
    AnalysisOnly,
}

impl RunMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Full => "full",
            Self::AnalysisOnly => "analysis_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CurriculumSizeHint {
    TinyFixture,
    Smoke,
    Full,
}

impl CurriculumSizeHint {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TinyFixture => "tiny_fixture",
            Self::Smoke => "smoke",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeacherConfig {
    pub backend: TeacherBackendKind,
    pub runtime: RuntimeConfig,
    pub local_model: LocalModelConfig,
}

impl TeacherConfig {
    pub fn runtime_descriptor(&self) -> RuntimeDescriptor {
        RuntimeDescriptor {
            backend: self.backend.clone(),
            runtime_path: self.runtime.runtime_path.clone(),
            server_binary: self.runtime.server_binary.clone(),
            model_path: self.local_model.model_path.clone(),
            endpoint: self.runtime.endpoint.clone(),
            enabled: self.runtime.enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeacherBackendKind {
    Mock,
    LocalLlm,
}

impl TeacherBackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::LocalLlm => "local_llm",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeConfig {
    pub enabled: bool,
    pub runtime_path: String,
    pub server_binary: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalModelConfig {
    pub model_path: String,
    pub context_size: u32,
    pub gpu_layers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeDescriptor {
    pub backend: TeacherBackendKind,
    pub runtime_path: String,
    pub server_binary: String,
    pub model_path: String,
    pub endpoint: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McmConfig {
    pub class_label: String,
    pub deterministic_only: bool,
    pub refusal_mode: String,
    pub memory_store: String,
    pub policy_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillManifest {
    pub manifest_version: String,
    pub skills: Vec<SkillManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillManifestEntry {
    pub skill_id: String,
    pub description: String,
    pub deterministic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillApprovals {
    pub approvals_version: String,
    pub approved_skill_ids: Vec<String>,
    pub blocked_skill_ids: Vec<String>,
}

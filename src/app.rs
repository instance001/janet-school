use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;

use crate::app_paths;
use crate::config::{AppConfig, TeacherBackendKind};
use crate::gui::{GuiShell, serve_gui_bridge, sync_gui_state};
use crate::mcm::{ExplicitMemoryStore, StudentEngine};
use crate::session::SessionService;

#[derive(Debug, Parser)]
#[command(name = "janet-school-rs")]
#[command(about = "Standalone Janet School research rig")]
pub struct Cli {
    #[arg(long, default_value = "config")]
    pub config_dir: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    ValidateConfig,
    InitSession {
        #[arg(long)]
        session_name: Option<String>,
    },
    GenerateCurriculum {
        #[arg(long)]
        session_name: Option<String>,
        #[arg(long, value_enum)]
        teacher_backend: Option<TeacherBackendArg>,
    },
    RunSession {
        #[arg(long)]
        session_name: Option<String>,
        #[arg(long, value_enum)]
        teacher_backend: Option<TeacherBackendArg>,
    },
    GuiAction {
        #[arg(long, value_enum)]
        action: GuiActionArg,
        #[arg(long)]
        session_name: Option<String>,
        #[arg(long, value_enum)]
        teacher_backend: Option<TeacherBackendArg>,
    },
    ServeGui {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        #[arg(long, default_value_t = false)]
        no_browser: bool,
    },
    SyncGuiState,
    InspectRuntime,
    PrintGuiShell,
    RunMcmPrompt {
        #[arg(long)]
        prompt: String,
    },
}

pub fn run() -> Result<()> {
    crate::util::init_tracing();
    let cli = Cli::parse();
    let app_root = app_paths::app_root();
    app_paths::ensure_app_layout(&app_root)
        .with_context(|| format!("failed to create app folders under {}", app_root.display()))?;
    let config_dir = app_paths::resolve(&app_root, &cli.config_dir);

    match cli.command {
        Command::ValidateConfig => {
            let config = load_config(&app_root, &config_dir)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        Command::InitSession { session_name } => {
            let config = load_config(&app_root, &config_dir)?;
            let service = SessionService::new(config);
            let created = service
                .initialize_session(session_name)
                .context("failed to initialize session")?;
            let config = load_config(&app_root, &config_dir)?;
            sync_gui_state(&config, &app_root)?;
            println!("{}", serde_json::to_string_pretty(&created)?);
        }
        Command::GenerateCurriculum {
            session_name,
            teacher_backend,
        } => {
            let config = load_config_with_override(&app_root, &config_dir, teacher_backend)?;
            let service = SessionService::new(config);
            let generated = service
                .initialize_and_generate_curriculum(session_name)
                .context("failed to generate curriculum")?;
            let config = load_config(&app_root, &config_dir)?;
            sync_gui_state(&config, &app_root)?;
            println!("{}", serde_json::to_string_pretty(&generated)?);
        }
        Command::RunSession {
            session_name,
            teacher_backend,
        } => {
            let config = load_config_with_override(&app_root, &config_dir, teacher_backend)?;
            let service = SessionService::new(config);
            let completed = service
                .run_generated_curriculum_session(session_name)
                .context("failed to run session")?;
            let config = load_config(&app_root, &config_dir)?;
            sync_gui_state(&config, &app_root)?;
            println!("{}", serde_json::to_string_pretty(&completed)?);
        }
        Command::GuiAction {
            action,
            session_name,
            teacher_backend,
        } => {
            let result = run_gui_action(
                &app_root,
                &config_dir,
                action,
                session_name,
                teacher_backend,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ServeGui {
            host,
            port,
            no_browser,
        } => {
            let hosted_by_chattycog =
                std::env::var("CHATTYCOG_HOSTED").ok().as_deref() == Some("1");
            serve_gui_bridge(
                &config_dir,
                &app_root,
                &host,
                port,
                !no_browser && !hosted_by_chattycog,
            )?;
        }
        Command::SyncGuiState => {
            let config = load_config(&app_root, &config_dir)?;
            let state = sync_gui_state(&config, &app_root)?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Command::InspectRuntime => {
            let config = load_config(&app_root, &config_dir)?;
            let runtime = config.teacher.runtime_descriptor();
            println!("{}", serde_json::to_string_pretty(&runtime)?);
        }
        Command::PrintGuiShell => {
            let shell = GuiShell::from_workspace(app_root.clone());
            info!("prepared GUI shell metadata");
            println!("{}", serde_json::to_string_pretty(&shell)?);
        }
        Command::RunMcmPrompt { prompt } => {
            let config = load_config(&app_root, &config_dir)?;
            let engine = StudentEngine::new(
                config.mcm,
                config.skill_manifest,
                config.skill_approvals,
                ExplicitMemoryStore::with_exact_entries([
                    (
                        "what color is the stop sign?".to_string(),
                        "red".to_string(),
                    ),
                    (
                        "two plus two".to_string(),
                        "4".to_string(),
                    ),
                ]),
            );
            let response = engine.answer("cli-session", "cli-item", &prompt);
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TeacherBackendArg {
    Mock,
    LocalLlm,
}

impl TeacherBackendArg {
    fn into_config_kind(self) -> TeacherBackendKind {
        match self {
            Self::Mock => TeacherBackendKind::Mock,
            Self::LocalLlm => TeacherBackendKind::LocalLlm,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GuiActionArg {
    SyncState,
    GenerateCurriculum,
    RunSession,
    PauseRun,
    ResumeRun,
    StopRun,
}

fn load_config(app_root: &std::path::Path, config_dir: &std::path::Path) -> Result<AppConfig> {
    let config = AppConfig::load_from_dir(config_dir)?;
    Ok(config.resolved_against(app_root))
}

fn load_config_with_override(
    app_root: &std::path::Path,
    config_dir: &std::path::Path,
    teacher_backend: Option<TeacherBackendArg>,
) -> Result<AppConfig> {
    let mut config = load_config(app_root, config_dir)?;
    if let Some(backend) = teacher_backend {
        config.teacher.backend = backend.into_config_kind();
    }
    Ok(config)
}

fn run_gui_action(
    app_root: &std::path::Path,
    config_dir: &std::path::Path,
    action: GuiActionArg,
    session_name: Option<String>,
    teacher_backend: Option<TeacherBackendArg>,
) -> Result<serde_json::Value> {
    match action {
        GuiActionArg::SyncState => {
            let config = load_config(app_root, config_dir)?;
            let state = sync_gui_state(&config, app_root)?;
            Ok(serde_json::to_value(state)?)
        }
        GuiActionArg::GenerateCurriculum => {
            let config = load_config_with_override(app_root, config_dir, teacher_backend)?;
            let service = SessionService::new(config);
            let generated = service
                .initialize_and_generate_curriculum(session_name)
                .context("failed to generate curriculum from gui action")?;
            let config = load_config(app_root, config_dir)?;
            sync_gui_state(&config, app_root)?;
            Ok(serde_json::to_value(generated)?)
        }
        GuiActionArg::RunSession => {
            let config = load_config_with_override(app_root, config_dir, teacher_backend)?;
            let service = SessionService::new(config);
            let completed = service
                .run_generated_curriculum_session(session_name)
                .context("failed to run session from gui action")?;
            let config = load_config(app_root, config_dir)?;
            sync_gui_state(&config, app_root)?;
            Ok(serde_json::to_value(completed)?)
        }
        GuiActionArg::StopRun => Err(anyhow::anyhow!(
            "stop-run is only available through the live GUI bridge host"
        )),
        GuiActionArg::PauseRun => Err(anyhow::anyhow!(
            "pause-run is only available through the live GUI bridge host"
        )),
        GuiActionArg::ResumeRun => Err(anyhow::anyhow!(
            "resume-run is only available through the live GUI bridge host"
        )),
    }
}

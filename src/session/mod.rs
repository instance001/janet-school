use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::analysis::AnalysisReport;
use crate::analysis;
use crate::config::AppConfig;
use crate::curriculum::{
    CurriculumBundle, CurriculumItem, CurriculumRequest, CurriculumSummary,
    CurriculumValidationReport,
};
use crate::mcm::{ExplicitMemoryStore, McmAnswer, StudentEngine};
use crate::memory::MemoryEvent;
use crate::skills::SkillEvent;
use crate::storage::JsonlWriter;
use crate::teacher;
use crate::telemetry::{AnomalyEvent, InteractionEvent, RefusalEvent};

pub struct SessionService {
    config: AppConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunControlSignal {
    Continue,
    Pause,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionProgressUpdate {
    pub phase: String,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub root_dir: Option<String>,
    pub total_items_expected: Option<usize>,
    pub total_items_completed: usize,
    pub latest_item_id: Option<String>,
    pub latest_item_type: Option<String>,
    pub latest_domain_id: Option<String>,
    pub latest_concept_id: Option<String>,
    pub latest_prompt: Option<String>,
    pub latest_expected_answer: Option<String>,
    pub latest_janet_answer: Option<String>,
    pub latest_correctness_judgment: Option<String>,
    pub latest_teacher_feedback: Option<String>,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub refusal_count: usize,
    pub anomaly_count: usize,
    pub probe_count: usize,
    pub memory_reads: usize,
    pub message: String,
    pub timestamp: String,
}

impl SessionService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn initialize_session(&self, session_name: Option<String>) -> Result<CreatedSession> {
        let run_id = Uuid::new_v4().to_string();
        let session_name = session_name.unwrap_or_else(|| "janet-session".to_string());
        let slug = crate::util::slugify(&session_name);
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::create_dir_all(&self.config.session.aggregated_dir).with_context(|| {
            format!(
                "failed to create aggregated directory {}",
                self.config.session.aggregated_dir
            )
        })?;
        let root = PathBuf::from(&self.config.session.sessions_dir)
            .join(format!("{timestamp}_{slug}_{run_id}"));

        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create session directory {}", root.display()))?;

        let manifest = SessionManifest {
            session_id: Uuid::new_v4().to_string(),
            run_id,
            session_name,
            created_at: Utc::now().to_rfc3339(),
            run_mode: self.config.session.default_run_mode.as_str().to_string(),
            teacher_backend_id: self.config.teacher.backend.as_str().to_string(),
            root_dir: root.to_string_lossy().to_string(),
        };

        self.write_initial_files(&root, &manifest)?;

        Ok(CreatedSession {
            manifest,
            files: required_output_paths(&root),
        })
    }

    pub fn initialize_and_generate_curriculum(
        &self,
        session_name: Option<String>,
    ) -> Result<GeneratedCurriculumSession> {
        self.initialize_and_generate_curriculum_with_progress(session_name, |_| {})
    }

    pub fn initialize_and_generate_curriculum_with_progress<F>(
        &self,
        session_name: Option<String>,
        mut on_progress: F,
    ) -> Result<GeneratedCurriculumSession>
    where
        F: FnMut(SessionProgressUpdate),
    {
        let created = self.initialize_session(session_name)?;
        on_progress(SessionProgressUpdate {
            phase: "session_initialized".to_string(),
            session_id: Some(created.manifest.session_id.clone()),
            run_id: Some(created.manifest.run_id.clone()),
            root_dir: Some(created.manifest.root_dir.clone()),
            total_items_expected: None,
            total_items_completed: 0,
            latest_item_id: None,
            latest_item_type: None,
            latest_domain_id: None,
            latest_concept_id: None,
            latest_prompt: None,
            latest_expected_answer: None,
            latest_janet_answer: None,
            latest_correctness_judgment: None,
            latest_teacher_feedback: None,
            correct_count: 0,
            incorrect_count: 0,
            refusal_count: 0,
            anomaly_count: 0,
            probe_count: 0,
            memory_reads: 0,
            message: "Session folder and manifest created.".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let request = CurriculumRequest::from_size_hint(
            created.manifest.session_id.clone(),
            created.manifest.run_mode.clone(),
            self.config.session.curriculum_size_hint.as_str(),
        );
        let backend = teacher::build_backend(&self.config.teacher);
        let (curriculum, teacher_call) = backend.generate_curriculum(&request)?;
        let validation = curriculum.validate()?;

        self.write_generated_curriculum(&created.manifest.root_dir, &curriculum, &validation)?;
        self.write_teacher_call(&created.manifest.root_dir, &teacher_call)?;
        self.update_session_summary(
            &created.manifest.root_dir,
            curriculum.summary(),
            teacher_call.teacher_backend_id.clone(),
        )?;

        on_progress(SessionProgressUpdate {
            phase: "curriculum_generated".to_string(),
            session_id: Some(created.manifest.session_id.clone()),
            run_id: Some(created.manifest.run_id.clone()),
            root_dir: Some(created.manifest.root_dir.clone()),
            total_items_expected: Some(curriculum.items.len()),
            total_items_completed: 0,
            latest_item_id: None,
            latest_item_type: None,
            latest_domain_id: None,
            latest_concept_id: None,
            latest_prompt: None,
            latest_expected_answer: None,
            latest_janet_answer: None,
            latest_correctness_judgment: None,
            latest_teacher_feedback: None,
            correct_count: 0,
            incorrect_count: 0,
            refusal_count: 0,
            anomaly_count: 0,
            probe_count: curriculum
                .items
                .iter()
                .filter(|item| item.item_type.starts_with("probe_"))
                .count(),
            memory_reads: 0,
            message: format!(
                "Curriculum generated with {} items across {} domains.",
                curriculum.items.len(),
                curriculum.domains.len()
            ),
            timestamp: Utc::now().to_rfc3339(),
        });

        Ok(GeneratedCurriculumSession {
            created,
            request,
            validation,
            curriculum_summary: curriculum.summary(),
        })
    }

    pub fn run_generated_curriculum_session(
        &self,
        session_name: Option<String>,
    ) -> Result<CompletedRunSession> {
        self.run_generated_curriculum_session_with_control(
            session_name,
            |_| {},
            || RunControlSignal::Continue,
        )
    }

    pub fn run_generated_curriculum_session_with_progress<F>(
        &self,
        session_name: Option<String>,
        on_progress: F,
    ) -> Result<CompletedRunSession>
    where
        F: FnMut(SessionProgressUpdate),
    {
        self.run_generated_curriculum_session_with_control(
            session_name,
            on_progress,
            || RunControlSignal::Continue,
        )
    }

    pub fn run_generated_curriculum_session_with_control<F, G>(
        &self,
        session_name: Option<String>,
        mut on_progress: F,
        mut should_stop: G,
    ) -> Result<CompletedRunSession>
    where
        F: FnMut(SessionProgressUpdate),
        G: FnMut() -> RunControlSignal,
    {
        let generated =
            self.initialize_and_generate_curriculum_with_progress(session_name, &mut on_progress)?;
        let root = PathBuf::from(&generated.created.manifest.root_dir);
        let curriculum = self.load_generated_curriculum(&root)?;
        let engine = StudentEngine::new(
            self.config.mcm.clone(),
            self.config.skill_manifest.clone(),
            self.config.skill_approvals.clone(),
            build_memory_store_from_curriculum(&curriculum),
        );

        let mut interactions_writer = JsonlWriter::new(root.join("interactions.jsonl"))?;
        let mut mcm_trace_writer = JsonlWriter::new(root.join("mcm_trace.jsonl"))?;
        let mut telemetry_writer = JsonlWriter::new(root.join("telemetry.jsonl"))?;
        let mut memory_writer = JsonlWriter::new(root.join("memory_events.jsonl"))?;
        let mut skill_writer = JsonlWriter::new(root.join("skill_events.jsonl"))?;
        let mut refusal_writer = JsonlWriter::new(root.join("refusal_events.jsonl"))?;
        let mut anomaly_writer = JsonlWriter::new(root.join("anomaly_events.jsonl"))?;
        let mut transfer_writer = JsonlWriter::new(root.join("transfer_probes.jsonl"))?;

        let mut stats = RunStats::default();
        let total_items_expected = curriculum.items.len();
        let mut completion_status = RunCompletionStatus::Completed;

        on_progress(SessionProgressUpdate {
            phase: "session_running".to_string(),
            session_id: Some(generated.created.manifest.session_id.clone()),
            run_id: Some(generated.created.manifest.run_id.clone()),
            root_dir: Some(generated.created.manifest.root_dir.clone()),
            total_items_expected: Some(total_items_expected),
            total_items_completed: 0,
            latest_item_id: None,
            latest_item_type: None,
            latest_domain_id: None,
            latest_concept_id: None,
            latest_prompt: None,
            latest_expected_answer: None,
            latest_janet_answer: None,
            latest_correctness_judgment: None,
            latest_teacher_feedback: None,
            correct_count: 0,
            incorrect_count: 0,
            refusal_count: 0,
            anomaly_count: 0,
            probe_count: 0,
            memory_reads: 0,
            message: "Starting Janet session execution.".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });

        for item in &curriculum.items {
            match wait_for_run_signal(
                &mut should_stop,
                &mut on_progress,
                &generated.created.manifest,
                total_items_expected,
                &stats,
            ) {
                RunControlSignal::Continue => {}
                RunControlSignal::Pause => continue,
                RunControlSignal::Stop => {
                    completion_status = RunCompletionStatus::Stopped;
                    break;
                }
            }

            let started = Instant::now();
            let answer = engine.answer(
                &generated.created.manifest.session_id,
                &item.item_id,
                &item.prompt,
            );
            let latency_ms = started.elapsed().as_millis() as u64;

            let interaction = build_interaction_event(
                &generated.created.manifest,
                item,
                &answer,
                latency_ms,
                &curriculum.version,
            );
            let skill_event = build_skill_event(
                &generated.created.manifest.session_id,
                item,
                &answer,
                &self.config.skill_approvals.approvals_version,
            );

            interactions_writer.append(&interaction)?;
            mcm_trace_writer.append(&answer.trace)?;
            skill_writer.append(&skill_event)?;
            telemetry_writer.append(&serde_json::json!({
                "event_type": "interaction",
                "payload": interaction
            }))?;
            telemetry_writer.append(&serde_json::json!({
                "event_type": "mcm_trace",
                "payload": answer.trace
            }))?;
            telemetry_writer.append(&serde_json::json!({
                "event_type": "skill_event",
                "payload": skill_event
            }))?;

            for memory_event in build_memory_events(&generated.created.manifest.session_id, item, &answer) {
                memory_writer.append(&memory_event)?;
                telemetry_writer.append(&serde_json::json!({
                    "event_type": "memory_event",
                    "payload": memory_event
                }))?;
                stats.memory_reads += 1;
            }

            if interaction.item_type.starts_with("probe_") {
                transfer_writer.append(&interaction)?;
                stats.probe_count += 1;
            }

            if let Some(refusal) = build_refusal_event(&generated.created.manifest.session_id, item, &answer) {
                refusal_writer.append(&refusal)?;
                telemetry_writer.append(&serde_json::json!({
                    "event_type": "refusal_event",
                    "payload": refusal
                }))?;
                stats.refusal_count += 1;
            }

            if let Some(anomaly) = build_anomaly_event(&generated.created.manifest.session_id, item, &interaction, &answer) {
                stats.anomaly_count += anomaly.anomaly_flags.len();
                anomaly_writer.append(&anomaly)?;
                telemetry_writer.append(&serde_json::json!({
                    "event_type": "anomaly_event",
                    "payload": anomaly
                }))?;
            }

            stats.total_items += 1;
            if interaction.correctness_judgment == "correct" {
                stats.correct_count += 1;
            } else {
                stats.incorrect_count += 1;
            }

            on_progress(SessionProgressUpdate {
                phase: "item_executed".to_string(),
                session_id: Some(generated.created.manifest.session_id.clone()),
                run_id: Some(generated.created.manifest.run_id.clone()),
                root_dir: Some(generated.created.manifest.root_dir.clone()),
                total_items_expected: Some(total_items_expected),
                total_items_completed: stats.total_items,
                latest_item_id: Some(item.item_id.clone()),
                latest_item_type: Some(item.item_type.clone()),
                latest_domain_id: Some(item.domain_id.clone()),
                latest_concept_id: Some(item.concept_id.clone()),
                latest_prompt: Some(item.prompt.clone()),
                latest_expected_answer: item.expected_answer.clone(),
                latest_janet_answer: interaction.janet_answer.clone(),
                latest_correctness_judgment: Some(interaction.correctness_judgment.clone()),
                latest_teacher_feedback: interaction.teacher_feedback.clone(),
                correct_count: stats.correct_count,
                incorrect_count: stats.incorrect_count,
                refusal_count: stats.refusal_count,
                anomaly_count: stats.anomaly_count,
                probe_count: stats.probe_count,
                memory_reads: stats.memory_reads,
                message: format!(
                    "Executed {} ({}/{})",
                    item.item_id, stats.total_items, total_items_expected
                ),
                timestamp: Utc::now().to_rfc3339(),
            });
        }

        self.finalize_session_summary(&root, &generated.curriculum_summary, &stats, completion_status)?;
        self.write_analysis_report(&root, &generated.created.manifest)?;

        on_progress(SessionProgressUpdate {
            phase: if completion_status == RunCompletionStatus::Stopped {
                "session_stopped".to_string()
            } else {
                "session_completed".to_string()
            },
            session_id: Some(generated.created.manifest.session_id.clone()),
            run_id: Some(generated.created.manifest.run_id.clone()),
            root_dir: Some(generated.created.manifest.root_dir.clone()),
            total_items_expected: Some(total_items_expected),
            total_items_completed: stats.total_items,
            latest_item_id: None,
            latest_item_type: None,
            latest_domain_id: None,
            latest_concept_id: None,
            latest_prompt: None,
            latest_expected_answer: None,
            latest_janet_answer: None,
            latest_correctness_judgment: None,
            latest_teacher_feedback: None,
            correct_count: stats.correct_count,
            incorrect_count: stats.incorrect_count,
            refusal_count: stats.refusal_count,
            anomaly_count: stats.anomaly_count,
            probe_count: stats.probe_count,
            memory_reads: stats.memory_reads,
            message: if completion_status == RunCompletionStatus::Stopped {
                "Session stopped by operator and partial analysis artifacts were written."
                    .to_string()
            } else {
                "Session execution and analysis completed.".to_string()
            },
            timestamp: Utc::now().to_rfc3339(),
        });

        Ok(CompletedRunSession {
            generated,
            run_stats: stats,
            completion_status: completion_status.as_str().to_string(),
        })
    }

    fn write_initial_files(&self, root: &Path, manifest: &SessionManifest) -> Result<()> {
        let session_config_path = root.join("session_config.json");
        fs::write(
            &session_config_path,
            serde_json::to_vec_pretty(&SessionConfigSnapshot {
                manifest: manifest.clone(),
                app: self.config.app.clone(),
                teacher: self.config.teacher.clone(),
                mcm: self.config.mcm.clone(),
                skill_manifest: self.config.skill_manifest.clone(),
                skill_approvals: self.config.skill_approvals.clone(),
            })?,
        )
        .with_context(|| format!("failed to write {}", session_config_path.display()))?;

        touch_jsonl(root.join("curriculum_generated.jsonl"))?;
        touch_jsonl(root.join("curriculum_validated.jsonl"))?;
        touch_jsonl(root.join("teacher_calls.jsonl"))?;
        touch_jsonl(root.join("interactions.jsonl"))?;
        touch_jsonl(root.join("mcm_trace.jsonl"))?;
        touch_jsonl(root.join("telemetry.jsonl"))?;
        touch_jsonl(root.join("memory_events.jsonl"))?;
        touch_jsonl(root.join("skill_events.jsonl"))?;
        touch_jsonl(root.join("refusal_events.jsonl"))?;
        touch_jsonl(root.join("transfer_probes.jsonl"))?;
        touch_jsonl(root.join("anomaly_events.jsonl"))?;

        let report = AnalysisReport {
            session_id: manifest.session_id.clone(),
            run_id: manifest.run_id.clone(),
            analysis_version: "0.1.0".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            confirmed_signals: Vec::new(),
            boundary_signals: Vec::new(),
            emergent_candidate_signals: Vec::new(),
            unknown_structure_candidates: Vec::new(),
            repeated_anomaly_clusters: Vec::new(),
            category_mismatch_clusters: Vec::new(),
            cross_session_summary: Vec::new(),
            caution_notes: vec!["Phase 1 placeholder report; no analysis run yet.".to_string()],
            recommended_next_probes: Vec::new(),
        };

        fs::write(
            root.join("analysis_report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(
            root.join("analysis_report.md"),
            "# Janet School Analysis Report\n\nPhase 1 placeholder report.\n",
        )?;
        fs::write(
            root.join("session_summary.json"),
            serde_json::to_vec_pretty(&SessionSummary::from_manifest(manifest))?,
        )?;

        Ok(())
    }

    fn write_generated_curriculum(
        &self,
        root_dir: &str,
        curriculum: &CurriculumBundle,
        validation: &CurriculumValidationReport,
    ) -> Result<()> {
        let root = PathBuf::from(root_dir);
        let mut generated_writer = JsonlWriter::new(root.join("curriculum_generated.jsonl"))?;
        generated_writer.append(curriculum)?;
        let mut validated_writer = JsonlWriter::new(root.join("curriculum_validated.jsonl"))?;
        validated_writer.append(validation)?;
        Ok(())
    }

    fn write_teacher_call(&self, root_dir: &str, record: &crate::teacher::TeacherCallRecord) -> Result<()> {
        let root = PathBuf::from(root_dir);
        let mut writer = JsonlWriter::new(root.join("teacher_calls.jsonl"))?;
        writer.append(record)?;
        Ok(())
    }

    fn load_generated_curriculum(&self, root: &Path) -> Result<CurriculumBundle> {
        let path = root.join("curriculum_generated.jsonl");
        let raw = fs::read_to_string(&path)?;
        let first = raw
            .lines()
            .find(|line| !line.trim().is_empty())
            .context("generated curriculum file is empty")?;
        Ok(serde_json::from_str(first)?)
    }

    fn update_session_summary(
        &self,
        root_dir: &str,
        curriculum_summary: CurriculumSummary,
        teacher_backend_id: String,
    ) -> Result<()> {
        let root = PathBuf::from(root_dir);
        let path = root.join("session_summary.json");
        let raw = fs::read_to_string(&path)?;
        let mut summary: SessionSummary = serde_json::from_str(&raw)?;
        summary.teacher_backend_id = teacher_backend_id;
        summary.curriculum_stats = serde_json::to_value(curriculum_summary)?;
        summary.notes = vec!["Curriculum generated and validated.".to_string()];
        fs::write(path, serde_json::to_vec_pretty(&summary)?)?;
        Ok(())
    }

    fn finalize_session_summary(
        &self,
        root: &Path,
        curriculum_summary: &CurriculumSummary,
        stats: &RunStats,
        completion_status: RunCompletionStatus,
    ) -> Result<()> {
        let path = root.join("session_summary.json");
        let raw = fs::read_to_string(&path)?;
        let mut summary: SessionSummary = serde_json::from_str(&raw)?;
        summary.completed_at = Some(Utc::now().to_rfc3339());
        summary.curriculum_stats = serde_json::to_value(curriculum_summary)?;
        summary.interaction_stats = serde_json::json!({
            "total_items": stats.total_items,
            "correct_count": stats.correct_count,
            "incorrect_count": stats.incorrect_count,
            "probe_count": stats.probe_count
        });
        summary.memory_stats = serde_json::json!({
            "memory_reads": stats.memory_reads,
            "memory_writes": 0
        });
        summary.skill_stats = serde_json::json!({
            "executed_skills": stats.total_items.saturating_sub(stats.refusal_count),
            "refusals_due_to_missing_or_insufficient_skill": stats.refusal_count
        });
        summary.refusal_stats = serde_json::json!({
            "refusal_count": stats.refusal_count
        });
        summary.anomaly_stats = serde_json::json!({
            "anomaly_flag_count": stats.anomaly_count
        });
        summary.notes = vec![match completion_status {
            RunCompletionStatus::Completed => {
                "Curriculum generated, executed, and logged.".to_string()
            }
            RunCompletionStatus::Stopped => {
                "Run was stopped by the operator after partial execution; artifacts reflect a partial session.".to_string()
            }
        }];
        fs::write(path, serde_json::to_vec_pretty(&summary)?)?;
        Ok(())
    }

    fn write_analysis_report(
        &self,
        root: &Path,
        manifest: &SessionManifest,
    ) -> Result<()> {
        let interactions = self.read_jsonl::<InteractionEvent>(&root.join("interactions.jsonl"))?;
        let refusals = self.read_jsonl::<RefusalEvent>(&root.join("refusal_events.jsonl"))?;
        let anomalies = self.read_jsonl::<AnomalyEvent>(&root.join("anomaly_events.jsonl"))?;
        let skill_events = self.read_jsonl::<SkillEvent>(&root.join("skill_events.jsonl"))?;
        let memory_events = self.read_jsonl::<MemoryEvent>(&root.join("memory_events.jsonl"))?;
        let prior_sessions = self.collect_prior_cross_session_inputs(&manifest.run_id)?;
        let report = analysis::build_report(
            &manifest.session_id,
            &manifest.run_id,
            &interactions,
            &refusals,
            &anomalies,
            &skill_events,
            &memory_events,
            &prior_sessions,
        );
        fs::write(
            root.join("analysis_report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(root.join("analysis_report.md"), analysis::render_markdown(&report))?;
        Ok(())
    }

    fn collect_prior_cross_session_inputs(
        &self,
        current_run_id: &str,
    ) -> Result<Vec<analysis::CrossSessionInput>> {
        let sessions_root = PathBuf::from(&self.config.session.sessions_dir);
        if !sessions_root.exists() {
            return Ok(Vec::new());
        }

        let mut prior = Vec::new();
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

            let summary_raw = fs::read_to_string(&summary_path)?;
            let summary: SessionSummary = serde_json::from_str(&summary_raw)?;
            if summary.run_id == current_run_id || summary.completed_at.is_none() {
                continue;
            }

            let analysis_path = path.join("analysis_report.json");
            let prior_analysis = if analysis_path.exists() {
                let raw = fs::read_to_string(analysis_path)?;
                serde_json::from_str::<AnalysisReport>(&raw).ok()
            } else {
                None
            };

            prior.push(analysis::CrossSessionInput {
                run_id: summary.run_id,
                run_mode: summary.run_mode,
                teacher_backend_id: summary.teacher_backend_id,
                total_items: summary
                    .interaction_stats
                    .get("total_items")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                refusal_count: summary
                    .refusal_stats
                    .get("refusal_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                anomaly_count: summary
                    .anomaly_stats
                    .get("anomaly_flag_count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                confirmed_count: prior_analysis
                    .as_ref()
                    .map(|report| report.confirmed_signals.len())
                    .unwrap_or(0),
                boundary_count: prior_analysis
                    .as_ref()
                    .map(|report| report.boundary_signals.len())
                    .unwrap_or(0),
                emergent_count: prior_analysis
                    .as_ref()
                    .map(|report| report.emergent_candidate_signals.len())
                    .unwrap_or(0),
            });
        }

        prior.sort_by(|left, right| left.run_id.cmp(&right.run_id));
        Ok(prior)
    }

    fn read_jsonl<T: for<'de> Deserialize<'de>>(&self, path: &Path) -> Result<Vec<T>> {
        let raw = fs::read_to_string(path)?;
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<Vec<T>, _>>()
            .map_err(Into::into)
    }
}

fn wait_for_run_signal<F, G>(
    should_stop: &mut G,
    on_progress: &mut F,
    manifest: &SessionManifest,
    total_items_expected: usize,
    stats: &RunStats,
) -> RunControlSignal
where
    F: FnMut(SessionProgressUpdate),
    G: FnMut() -> RunControlSignal,
{
    let mut pause_emitted = false;

    loop {
        match should_stop() {
            RunControlSignal::Continue => return RunControlSignal::Continue,
            RunControlSignal::Stop => return RunControlSignal::Stop,
            RunControlSignal::Pause => {
                if !pause_emitted {
                    pause_emitted = true;
                    on_progress(SessionProgressUpdate {
                        phase: "session_paused".to_string(),
                        session_id: Some(manifest.session_id.clone()),
                        run_id: Some(manifest.run_id.clone()),
                        root_dir: Some(manifest.root_dir.clone()),
                        total_items_expected: Some(total_items_expected),
                        total_items_completed: stats.total_items,
                        latest_item_id: None,
                        latest_item_type: None,
                        latest_domain_id: None,
                        latest_concept_id: None,
                        latest_prompt: None,
                        latest_expected_answer: None,
                        latest_janet_answer: None,
                        latest_correctness_judgment: None,
                        latest_teacher_feedback: None,
                        correct_count: stats.correct_count,
                        incorrect_count: stats.incorrect_count,
                        refusal_count: stats.refusal_count,
                        anomaly_count: stats.anomaly_count,
                        probe_count: stats.probe_count,
                        memory_reads: stats.memory_reads,
                        message: "Session paused by operator. Waiting for resume or stop."
                            .to_string(),
                        timestamp: Utc::now().to_rfc3339(),
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }
    }
}

fn touch_jsonl(path: PathBuf) -> Result<()> {
    let writer = JsonlWriter::new(path)?;
    writer.flush()?;
    Ok(())
}

fn required_output_paths(root: &Path) -> Vec<String> {
    [
        "session_config.json",
        "curriculum_generated.jsonl",
        "curriculum_validated.jsonl",
        "teacher_calls.jsonl",
        "interactions.jsonl",
        "mcm_trace.jsonl",
        "telemetry.jsonl",
        "memory_events.jsonl",
        "skill_events.jsonl",
        "refusal_events.jsonl",
        "transfer_probes.jsonl",
        "anomaly_events.jsonl",
        "analysis_report.json",
        "analysis_report.md",
        "session_summary.json",
    ]
    .into_iter()
    .map(|name| root.join(name).to_string_lossy().to_string())
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionManifest {
    pub session_id: String,
    pub run_id: String,
    pub session_name: String,
    pub created_at: String,
    pub run_mode: String,
    pub teacher_backend_id: String,
    pub root_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreatedSession {
    pub manifest: SessionManifest,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionConfigSnapshot {
    pub manifest: SessionManifest,
    pub app: crate::config::AppMetadata,
    pub teacher: crate::config::TeacherConfig,
    pub mcm: crate::config::McmConfig,
    pub skill_manifest: crate::config::SkillManifest,
    pub skill_approvals: crate::config::SkillApprovals,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionSummary {
    pub session_id: String,
    pub run_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub run_mode: String,
    pub teacher_backend_id: String,
    pub curriculum_stats: serde_json::Value,
    pub interaction_stats: serde_json::Value,
    pub memory_stats: serde_json::Value,
    pub skill_stats: serde_json::Value,
    pub refusal_stats: serde_json::Value,
    pub anomaly_stats: serde_json::Value,
    pub analysis_artifact_paths: Vec<String>,
    pub notes: Vec<String>,
}

impl SessionSummary {
    fn from_manifest(manifest: &SessionManifest) -> Self {
        Self {
            session_id: manifest.session_id.clone(),
            run_id: manifest.run_id.clone(),
            started_at: manifest.created_at.clone(),
            completed_at: None,
            run_mode: manifest.run_mode.clone(),
            teacher_backend_id: manifest.teacher_backend_id.clone(),
            curriculum_stats: serde_json::json!({}),
            interaction_stats: serde_json::json!({}),
            memory_stats: serde_json::json!({}),
            skill_stats: serde_json::json!({}),
            refusal_stats: serde_json::json!({}),
            anomaly_stats: serde_json::json!({}),
            analysis_artifact_paths: vec![
                "analysis_report.json".to_string(),
                "analysis_report.md".to_string(),
            ],
            notes: vec!["Phase 1 placeholder summary.".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeneratedCurriculumSession {
    pub created: CreatedSession,
    pub request: CurriculumRequest,
    pub validation: CurriculumValidationReport,
    pub curriculum_summary: CurriculumSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompletedRunSession {
    pub generated: GeneratedCurriculumSession,
    pub run_stats: RunStats,
    pub completion_status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RunStats {
    pub total_items: usize,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub refusal_count: usize,
    pub anomaly_count: usize,
    pub probe_count: usize,
    pub memory_reads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunCompletionStatus {
    Completed,
    Stopped,
}

impl RunCompletionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Stopped => "stopped",
        }
    }
}

fn build_memory_store_from_curriculum(curriculum: &CurriculumBundle) -> ExplicitMemoryStore {
    let mut entries = vec![
        ("what color is the stop sign?".to_string(), "red".to_string()),
        ("two plus two".to_string(), "4".to_string()),
    ];

    entries.extend(
        curriculum
            .items
            .iter()
            .filter(|item| item.item_type == "teaching")
            .filter(|item| {
                item.expected_skills.len() == 1
                    && item
                        .expected_skills
                        .iter()
                        .any(|skill| skill == "exact_match_lookup")
            })
            .filter_map(|item| {
                item.expected_answer
                    .as_ref()
                    .map(|expected| (item.prompt.to_ascii_lowercase(), expected.clone()))
            }),
    );

    ExplicitMemoryStore::with_exact_entries(entries)
}

fn build_interaction_event(
    manifest: &SessionManifest,
    item: &CurriculumItem,
    answer: &McmAnswer,
    latency_ms: u64,
    curriculum_version: &str,
) -> InteractionEvent {
    let correctness = determine_correctness(item, answer.answer.as_deref());
    let structure_fit = determine_structure_fit(item, answer, &correctness);
    let anomaly_flags = determine_anomaly_flags(item, answer, &correctness, &structure_fit);

    InteractionEvent {
        event_id: Uuid::new_v4().to_string(),
        session_id: manifest.session_id.clone(),
        run_id: manifest.run_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        curriculum_version: curriculum_version.to_string(),
        item_id: item.item_id.clone(),
        domain_id: item.domain_id.clone(),
        concept_id: item.concept_id.clone(),
        item_type: item.item_type.clone(),
        teacher_backend_id: manifest.teacher_backend_id.clone(),
        teacher_prompt: None,
        generated_question: item.prompt.clone(),
        intended_structure: item.intended_relations.clone(),
        expected_answer: item.expected_answer.clone(),
        janet_answer: answer.answer.clone(),
        correctness_judgment: correctness,
        teacher_feedback: None,
        structure_fit,
        anomaly_flags,
        latency_ms,
        raw_event_trace_hash: None,
        code_version_hash: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

fn build_skill_event(
    session_id: &str,
    item: &CurriculumItem,
    answer: &McmAnswer,
    approvals_version: &str,
) -> SkillEvent {
    SkillEvent {
        skill_event_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        item_id: item.item_id.clone(),
        candidate_skills: answer.trace.candidate_skills.clone(),
        approved_skills: answer.trace.approved_skills.clone(),
        blocked_skills: answer.trace.blocked_skills.clone(),
        executed_skill: answer.trace.executed_skill.clone(),
        approval_ledger_version: approvals_version.to_string(),
        reason: answer
            .trace
            .refusal_reason
            .clone()
            .unwrap_or_else(|| "deterministic skill path executed".to_string()),
    }
}

fn build_memory_events(session_id: &str, item: &CurriculumItem, answer: &McmAnswer) -> Vec<MemoryEvent> {
    answer
        .trace
        .memory_reads
        .iter()
        .map(|key| MemoryEvent {
            memory_event_id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            operation: "read".to_string(),
            memory_scope: "explicit_exact_answers".to_string(),
            key: key.clone(),
            value_before: None,
            value_after: None,
            reason: format!("lookup during item {}", item.item_id),
            source_item_id: Some(item.item_id.clone()),
            approved_by_policy: true,
        })
        .collect()
}

fn build_refusal_event(session_id: &str, item: &CurriculumItem, answer: &McmAnswer) -> Option<RefusalEvent> {
    let reason = answer.trace.refusal_reason.clone()?;
    Some(RefusalEvent {
        refusal_event_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        item_id: item.item_id.clone(),
        reason,
        uncertainty_state: answer.trace.uncertainty_state.clone(),
        candidate_next_steps: vec!["review curriculum item or approve additional deterministic skill".to_string()],
        policy_trace: answer.trace.policy_checks.clone(),
    })
}

fn build_anomaly_event(
    session_id: &str,
    item: &CurriculumItem,
    interaction: &InteractionEvent,
    answer: &McmAnswer,
) -> Option<AnomalyEvent> {
    if interaction.anomaly_flags.is_empty() {
        return None;
    }

    Some(AnomalyEvent {
        anomaly_event_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        item_id: item.item_id.clone(),
        structure_fit: interaction.structure_fit.clone(),
        anomaly_flags: interaction.anomaly_flags.clone(),
        observed_structure: answer.trace.surface_features.clone(),
        structure_fit_explanation: format!(
            "Correctness={}, final_mode={}, expected_skills={}",
            interaction.correctness_judgment,
            answer.trace.final_mode,
            item.expected_skills.join(",")
        ),
        supporting_trace_ids: vec![answer.trace.trace_id.clone()],
    })
}

fn determine_correctness(item: &CurriculumItem, answer: Option<&str>) -> String {
    let Some(answer) = answer.map(normalize_value) else {
        return "refused".to_string();
    };

    if let Some(expected) = item.expected_answer.as_deref().map(normalize_value)
        && answer == expected
    {
        return "correct".to_string();
    }

    if item
        .acceptable_answers
        .iter()
        .any(|candidate| normalize_value(candidate) == answer)
    {
        "correct".to_string()
    } else {
        "incorrect".to_string()
    }
}

fn determine_structure_fit(item: &CurriculumItem, answer: &McmAnswer, correctness: &str) -> String {
    match correctness {
        "correct" => {
            if let Some(executed) = answer.trace.executed_skill.as_deref() {
                if item.expected_skills.iter().any(|skill| skill == executed) {
                    "matched".to_string()
                } else {
                    "partial_match".to_string()
                }
            } else {
                "unknown".to_string()
            }
        }
        "refused" => {
            if item.probe_role == "boundary" {
                "partial_match".to_string()
            } else {
                "mismatch".to_string()
            }
        }
        _ => "mismatch".to_string(),
    }
}

fn determine_anomaly_flags(
    item: &CurriculumItem,
    answer: &McmAnswer,
    correctness: &str,
    structure_fit: &str,
) -> Vec<String> {
    let mut flags = Vec::new();

    if correctness == "refused" && item.item_type == "teaching" {
        flags.push("unexpected_refusal".to_string());
    }
    if correctness == "incorrect" && item.item_type == "teaching" {
        flags.push("unexpected_failure".to_string());
    }
    if correctness == "correct"
        && answer
            .trace
            .executed_skill
            .as_ref()
            .is_some_and(|skill| !item.expected_skills.iter().any(|expected| expected == skill))
    {
        flags.push("unexpected_skill_path".to_string());
    }
    if item.probe_role == "boundary" && correctness != "correct" {
        flags.push("boundary_pressure".to_string());
    }
    if structure_fit == "mismatch" {
        flags.push("structure_mismatch".to_string());
    }

    flags
}

fn normalize_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::env;
    use std::net::TcpListener;
    use std::thread;

    use crate::config::{
        AppConfig, AppMetadata, CurriculumSizeHint, LocalModelConfig, McmConfig, RunMode,
        RuntimeConfig, SessionConfig, SkillApprovals, SkillManifest, TeacherBackendKind,
        TeacherConfig,
    };
    use crate::telemetry::{InteractionEvent, RefusalEvent};
    use serde::Deserialize;
    use tiny_http::{Method, Response, Server, StatusCode};

    #[test]
    fn mock_run_writes_teacher_calls_and_telemetry_artifacts() {
        let root = unique_test_root("mock_run_artifacts");
        let config = test_config(&root, CurriculumSizeHint::Smoke, TeacherBackendKind::Mock, None);
        let service = SessionService::new(config);

        let completed = service
            .run_generated_curriculum_session(Some("Mock Run Artifacts".to_string()))
            .expect("mock run should complete");
        let session_root = PathBuf::from(&completed.generated.created.manifest.root_dir);

        let teacher_calls = read_jsonl::<crate::teacher::TeacherCallRecord>(&session_root.join("teacher_calls.jsonl"));
        let interactions = read_jsonl::<InteractionEvent>(&session_root.join("interactions.jsonl"));
        let telemetry = fs::read_to_string(session_root.join("telemetry.jsonl"))
            .expect("telemetry file should be readable");
        let memory_events = fs::read_to_string(session_root.join("memory_events.jsonl"))
            .expect("memory events file should be readable");

        assert_eq!(teacher_calls.len(), 1);
        assert_eq!(teacher_calls[0].teacher_backend_id, "mock");
        assert_eq!(interactions.len(), completed.run_stats.total_items);
        assert!(telemetry.contains("\"event_type\":\"interaction\""));
        assert!(telemetry.contains("\"event_type\":\"skill_event\""));
        assert!(!memory_events.contains("\"operation\":\"write\""));

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    #[test]
    fn blocked_skills_log_refusals_without_memory_writes() {
        let root = unique_test_root("blocked_skill_run");
        let config = test_config(
            &root,
            CurriculumSizeHint::TinyFixture,
            TeacherBackendKind::Mock,
            Some(vec!["option_match_selector".to_string()]),
        );
        let service = SessionService::new(config);

        let completed = service
            .run_generated_curriculum_session(Some("Blocked Skill Run".to_string()))
            .expect("blocked-skill run should complete with refusals");
        let session_root = PathBuf::from(&completed.generated.created.manifest.root_dir);

        let refusals = read_jsonl::<RefusalEvent>(&session_root.join("refusal_events.jsonl"));
        let memory_events = fs::read_to_string(session_root.join("memory_events.jsonl"))
            .expect("memory events file should be readable");
        let summary_raw = fs::read_to_string(session_root.join("session_summary.json"))
            .expect("summary should be readable");
        let summary: SessionSummary =
            serde_json::from_str(&summary_raw).expect("summary json should parse");

        assert!(!refusals.is_empty());
        assert!(completed.run_stats.refusal_count >= refusals.len());
        assert!(!memory_events.contains("\"operation\":\"write\""));
        assert_eq!(summary.refusal_stats["refusal_count"], serde_json::json!(completed.run_stats.refusal_count));
        assert_eq!(summary.memory_stats["memory_writes"], serde_json::json!(0));

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    #[test]
    fn stop_request_preserves_partial_run_and_marks_summary() {
        let root = unique_test_root("stopped_run");
        let config = test_config(&root, CurriculumSizeHint::Smoke, TeacherBackendKind::Mock, None);
        let service = SessionService::new(config);
        let should_stop = Cell::new(false);

        let completed = service
            .run_generated_curriculum_session_with_control(
                Some("Stopped Run".to_string()),
                |progress| {
                    if progress.phase == "item_executed" && progress.total_items_completed >= 1 {
                        should_stop.set(true);
                    }
                },
                || {
                    if should_stop.get() {
                        RunControlSignal::Stop
                    } else {
                        RunControlSignal::Continue
                    }
                },
            )
            .expect("stopped run should finalize cleanly");
        let session_root = PathBuf::from(&completed.generated.created.manifest.root_dir);
        let summary_raw = fs::read_to_string(session_root.join("session_summary.json"))
            .expect("summary should be readable");
        let summary: SessionSummary =
            serde_json::from_str(&summary_raw).expect("summary json should parse");
        let interactions = read_jsonl::<InteractionEvent>(&session_root.join("interactions.jsonl"));

        assert_eq!(completed.completion_status, "stopped");
        assert!(completed.run_stats.total_items >= 1);
        assert!(completed.run_stats.total_items < 30);
        assert_eq!(interactions.len(), completed.run_stats.total_items);
        assert!(summary.notes[0].contains("partial execution"));

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    #[test]
    fn later_run_analysis_includes_cross_session_summary_from_prior_runs() {
        let root = unique_test_root("cross_session_summary");
        let config = test_config(&root, CurriculumSizeHint::Smoke, TeacherBackendKind::Mock, None);
        let service = SessionService::new(config);

        let _first = service
            .run_generated_curriculum_session(Some("Cross Session One".to_string()))
            .expect("first run should complete");
        let second = service
            .run_generated_curriculum_session(Some("Cross Session Two".to_string()))
            .expect("second run should complete");

        let second_root = PathBuf::from(&second.generated.created.manifest.root_dir);
        let analysis_raw = fs::read_to_string(second_root.join("analysis_report.json"))
            .expect("analysis report should be readable");
        let report: AnalysisReport =
            serde_json::from_str(&analysis_raw).expect("analysis report should parse");

        assert!(!report.cross_session_summary.is_empty());
        assert!(
            report
                .cross_session_summary
                .iter()
                .any(|line| line.contains("Cross-session view covers 2 runs"))
        );
        assert!(
            report
                .cross_session_summary
                .iter()
                .all(|line| !line.contains("Single-session report only"))
        );

        let markdown = fs::read_to_string(second_root.join("analysis_report.md"))
            .expect("analysis markdown should be readable");
        assert!(markdown.contains("## Cross-Session Summary"));

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    #[test]
    fn completed_run_writes_analysis_report_with_required_sections() {
        let root = unique_test_root("analysis_report_sections");
        let config = test_config(&root, CurriculumSizeHint::Smoke, TeacherBackendKind::Mock, None);
        let service = SessionService::new(config);

        let completed = service
            .run_generated_curriculum_session(Some("Analysis Report Sections".to_string()))
            .expect("run should complete");
        let session_root = PathBuf::from(&completed.generated.created.manifest.root_dir);
        let markdown = fs::read_to_string(session_root.join("analysis_report.md"))
            .expect("analysis markdown should be readable");

        assert!(markdown.contains("## Confirmed Signals"));
        assert!(markdown.contains("## Boundary Signals"));
        assert!(markdown.contains("## Emergent Candidate Signals"));
        assert!(markdown.contains("## Unknown Structure Candidates"));
        assert!(markdown.contains("## Repeated Anomaly Clusters"));
        assert!(markdown.contains("## Category Mismatch Clusters"));
        assert!(markdown.contains("## Cross-Session Summary"));
        assert!(markdown.contains("## Caution Notes"));
        assert!(markdown.contains("## Recommended Next Probes"));

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    #[test]
    fn local_teacher_run_writes_local_teacher_call_record() {
        let server = spawn_fake_local_teacher_server();
        let root = unique_test_root("local_teacher_artifacts");
        let mut config = test_config(&root, CurriculumSizeHint::Smoke, TeacherBackendKind::LocalLlm, None);
        config.teacher.runtime.enabled = false;
        config.teacher.runtime.endpoint = server.endpoint.clone();
        config.teacher.local_model.model_path = "models/local-test.gguf".to_string();
        let service = SessionService::new(config);

        let completed = service
            .run_generated_curriculum_session(Some("Local Teacher Artifacts".to_string()))
            .expect("local teacher run should complete");
        let session_root = PathBuf::from(&completed.generated.created.manifest.root_dir);
        let teacher_calls = read_jsonl::<crate::teacher::TeacherCallRecord>(&session_root.join("teacher_calls.jsonl"));

        assert_eq!(teacher_calls.len(), 1);
        assert_eq!(teacher_calls[0].teacher_backend_id, "local_llm");
        assert_eq!(teacher_calls[0].operation, "generate_curriculum");
        assert_eq!(
            teacher_calls[0]
                .runtime_config
                .get("endpoint_ready")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            teacher_calls[0]
                .runtime_config
                .get("launched_runtime")
                .and_then(|value| value.as_bool()),
            Some(false)
        );

        fs::remove_dir_all(&root).expect("test output should clean up");
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("janet-school-{label}-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root should be created");
        root
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
                            "text": "{\"rationale\":\"session local test outline\",\"domains\":[{\"domain_id\":\"attention_discrimination\",\"domain_name\":\"Attention and Discrimination\",\"concepts\":[{\"concept_id\":\"attention_discrimination_visual_notice\",\"concept_name\":\"visual notice\"},{\"concept_id\":\"attention_discrimination_signal_pickout\",\"concept_name\":\"signal pickout\"}]},{\"domain_id\":\"matching_difference\",\"domain_name\":\"Matching and Difference\",\"concepts\":[{\"concept_id\":\"matching_difference_same_vs_different\",\"concept_name\":\"same vs different\"},{\"concept_id\":\"matching_difference_exact_match\",\"concept_name\":\"exact match\"}]},{\"domain_id\":\"sequencing_order\",\"domain_name\":\"Sequencing and Order\",\"concepts\":[{\"concept_id\":\"sequencing_order_before_after\",\"concept_name\":\"before after\"},{\"concept_id\":\"sequencing_order_first_last\",\"concept_name\":\"first last\"}]},{\"domain_id\":\"quantity_comparison\",\"domain_name\":\"Quantity and Comparison\",\"concepts\":[{\"concept_id\":\"quantity_comparison_more_less\",\"concept_name\":\"more less\"},{\"concept_id\":\"quantity_comparison_equal_compare\",\"concept_name\":\"equal compare\"}]},{\"domain_id\":\"spatial_reasoning\",\"domain_name\":\"Spatial Reasoning\",\"concepts\":[{\"concept_id\":\"spatial_reasoning_left_right\",\"concept_name\":\"left right\"},{\"concept_id\":\"spatial_reasoning_inside_outside\",\"concept_name\":\"inside outside\"}]}]}"
                        }],
                        "usage": {
                            "prompt_tokens": 55,
                            "completion_tokens": 89
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

    fn test_config(
        root: &Path,
        curriculum_size_hint: CurriculumSizeHint,
        backend: TeacherBackendKind,
        blocked_skill_ids: Option<Vec<String>>,
    ) -> AppConfig {
        let data_dir = root.join("data");
        let sessions_dir = data_dir.join("sessions");
        let aggregated_dir = data_dir.join("aggregated");
        fs::create_dir_all(&sessions_dir).expect("sessions dir should exist");
        fs::create_dir_all(&aggregated_dir).expect("aggregated dir should exist");

        let blocked_skill_ids = blocked_skill_ids.unwrap_or_default();
        let approved_skill_ids = [
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
        .filter(|skill_id| !blocked_skill_ids.iter().any(|blocked| blocked == skill_id))
        .map(str::to_string)
        .collect::<Vec<_>>();

        let session = SessionConfig {
            default_run_mode: RunMode::Smoke,
            sessions_dir: sessions_dir.to_string_lossy().to_string(),
            aggregated_dir: aggregated_dir.to_string_lossy().to_string(),
            curriculum_size_hint,
        };

        AppConfig {
            app: AppMetadata {
                app_name: "Janet School".to_string(),
                version: "0.1.0".to_string(),
                environment: "test".to_string(),
                docs_dir: "docs".to_string(),
                data_dir: data_dir.to_string_lossy().to_string(),
                web_dir: "web".to_string(),
                show_splash: true,
                splash_duration_ms: 3000,
                session: session.clone(),
            },
            session,
            teacher: TeacherConfig {
                backend,
                runtime: RuntimeConfig {
                    enabled: false,
                    runtime_path: "runtime".to_string(),
                    server_binary: "runtime/llama-server.exe".to_string(),
                    endpoint: "http://127.0.0.1:8080/v1".to_string(),
                },
                local_model: LocalModelConfig {
                    model_path: "models/placeholder.gguf".to_string(),
                    context_size: 4096,
                    gpu_layers: 0,
                },
            },
            mcm: McmConfig {
                class_label: "janet".to_string(),
                deterministic_only: true,
                refusal_mode: "strict".to_string(),
                memory_store: "explicit_exact_answers".to_string(),
                policy_version: "test".to_string(),
            },
            skill_manifest: SkillManifest {
                manifest_version: "test".to_string(),
                skills: vec![
                    skill_entry("option_match_selector"),
                    skill_entry("ordered_relation_compare"),
                    skill_entry("same_different_compare"),
                    skill_entry("first_last_selector"),
                    skill_entry("more_less_compare"),
                    skill_entry("equal_compare"),
                    skill_entry("left_right_selector"),
                    skill_entry("inside_outside_selector"),
                    skill_entry("exact_match_lookup"),
                ],
            },
            skill_approvals: SkillApprovals {
                approvals_version: "test".to_string(),
                approved_skill_ids,
                blocked_skill_ids,
            },
        }
    }

    fn skill_entry(skill_id: &str) -> crate::config::SkillManifestEntry {
        crate::config::SkillManifestEntry {
            skill_id: skill_id.to_string(),
            description: format!("{skill_id} for tests"),
            deterministic: true,
        }
    }

    fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
        fs::read_to_string(path)
            .expect("jsonl file should be readable")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("jsonl row should parse"))
            .collect()
    }
}

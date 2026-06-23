use std::collections::HashMap;

use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::memory::MemoryEvent;
use crate::skills::SkillEvent;
use crate::telemetry::{AnomalyEvent, InteractionEvent, RefusalEvent};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisReport {
    pub session_id: String,
    pub run_id: String,
    pub analysis_version: String,
    pub generated_at: String,
    pub confirmed_signals: Vec<Signal>,
    pub boundary_signals: Vec<Signal>,
    pub emergent_candidate_signals: Vec<Signal>,
    pub unknown_structure_candidates: Vec<Signal>,
    pub repeated_anomaly_clusters: Vec<Cluster>,
    pub category_mismatch_clusters: Vec<Cluster>,
    pub cross_session_summary: Vec<String>,
    pub caution_notes: Vec<String>,
    pub recommended_next_probes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Signal {
    pub signal_id: String,
    pub signal_type: String,
    pub summary: String,
    pub supporting_event_ids: Vec<String>,
    pub explanation: String,
    pub confidence: String,
    pub requires_human_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Cluster {
    pub cluster_id: String,
    pub cluster_type: String,
    pub item_ids: Vec<String>,
    pub supporting_event_ids: Vec<String>,
    pub count: u32,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrossSessionInput {
    pub run_id: String,
    pub run_mode: String,
    pub teacher_backend_id: String,
    pub total_items: usize,
    pub refusal_count: usize,
    pub anomaly_count: usize,
    pub confirmed_count: usize,
    pub boundary_count: usize,
    pub emergent_count: usize,
}

pub fn build_report(
    session_id: &str,
    run_id: &str,
    interactions: &[InteractionEvent],
    refusals: &[RefusalEvent],
    anomalies: &[AnomalyEvent],
    skill_events: &[SkillEvent],
    memory_events: &[MemoryEvent],
    prior_sessions: &[CrossSessionInput],
) -> AnalysisReport {
    let skill_by_item: HashMap<&str, &SkillEvent> = skill_events
        .iter()
        .map(|event| (event.item_id.as_str(), event))
        .collect();
    let memory_read_items: HashMap<&str, usize> =
        memory_events.iter().fold(HashMap::new(), |mut acc, event| {
            if let Some(item_id) = event.source_item_id.as_deref() {
                *acc.entry(item_id).or_insert(0) += 1;
            }
            acc
        });

    let confirmed_signals = interactions
        .iter()
        .filter(|event| event.correctness_judgment == "correct" && event.structure_fit == "matched")
        .take(6)
        .map(|event| Signal {
            signal_id: Uuid::new_v4().to_string(),
            signal_type: confirmed_signal_type(event, &skill_by_item, &memory_read_items),
            summary: confirmed_signal_summary(event, &skill_by_item, &memory_read_items),
            supporting_event_ids: vec![event.event_id.clone()],
            explanation: confirmed_signal_explanation(event, &skill_by_item, &memory_read_items),
            confidence: "medium".to_string(),
            requires_human_review: false,
        })
        .collect::<Vec<_>>();

    let boundary_signals = anomalies
        .iter()
        .filter(|event| event.anomaly_flags.iter().any(|flag| flag == "boundary_pressure"))
        .map(|event| Signal {
            signal_id: Uuid::new_v4().to_string(),
            signal_type: "boundary_signal".to_string(),
            summary: format!("Boundary pressure on item {}.", event.item_id),
            supporting_event_ids: event.supporting_trace_ids.clone(),
            explanation: event.structure_fit_explanation.clone(),
            confidence: "medium".to_string(),
            requires_human_review: true,
        })
        .collect::<Vec<_>>();

    let repeated_unexpected_refusals = cluster_by_flag(interactions, "unexpected_refusal");
    let emergent_candidate_signals = if repeated_unexpected_refusals
        .iter()
        .any(|cluster| cluster.count >= 3)
    {
        repeated_unexpected_refusals
            .iter()
            .filter(|cluster| cluster.count >= 3)
            .map(|cluster| Signal {
                signal_id: Uuid::new_v4().to_string(),
                signal_type: "emergent_candidate_signal".to_string(),
                summary: format!(
                    "Repeated refusal cluster in domain sample: {} items.",
                    cluster.count
                ),
                supporting_event_ids: cluster.supporting_event_ids.clone(),
                explanation:
                    "Current curriculum labels may be too broad for the available deterministic skill set."
                        .to_string(),
                confidence: "low".to_string(),
                requires_human_review: true,
            })
            .collect()
    } else {
        Vec::new()
    };

    let unknown_structure_candidates = interactions
        .iter()
        .filter(|event| event.structure_fit == "partial_match")
        .take(3)
        .map(|event| Signal {
            signal_id: Uuid::new_v4().to_string(),
            signal_type: "unknown_structure_candidate".to_string(),
            summary: format!("Partial fit observed on {}.", event.item_id),
            supporting_event_ids: vec![event.event_id.clone()],
            explanation: "Observed behavior fits only part of the intended structure.".to_string(),
            confidence: "low".to_string(),
            requires_human_review: true,
        })
        .collect();

    let repeated_anomaly_clusters = build_anomaly_clusters(interactions);
    let category_mismatch_clusters = build_mismatch_clusters(interactions);
    let caution_notes = build_caution_notes(interactions, refusals, anomalies);
    let recommended_next_probes = build_next_probes(interactions, refusals);
    let confirmed_memory_count = confirmed_signals
        .iter()
        .filter(|signal| signal.signal_type == "confirmed_memory_signal")
        .count();
    let confirmed_skill_count = confirmed_signals
        .iter()
        .filter(|signal| signal.signal_type == "confirmed_skill_signal")
        .count();

    let cross_session_summary = build_cross_session_summary(
        run_id,
        confirmed_memory_count,
        confirmed_skill_count,
        &confirmed_signals,
        &boundary_signals,
        &emergent_candidate_signals,
        prior_sessions,
    );

    AnalysisReport {
        session_id: session_id.to_string(),
        run_id: run_id.to_string(),
        analysis_version: "0.1.0".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        confirmed_signals,
        boundary_signals,
        emergent_candidate_signals,
        unknown_structure_candidates,
        repeated_anomaly_clusters,
        category_mismatch_clusters,
        cross_session_summary,
        caution_notes,
        recommended_next_probes,
    }
}

pub fn render_markdown(report: &AnalysisReport) -> String {
    let mut out = String::new();
    out.push_str("# Janet School Analysis Report\n\n");
    out.push_str("This report uses provisional language and requires human review for interpretive claims.\n\n");
    render_signal_section(&mut out, "Confirmed Signals", &report.confirmed_signals);
    render_signal_section(&mut out, "Boundary Signals", &report.boundary_signals);
    render_signal_section(
        &mut out,
        "Emergent Candidate Signals",
        &report.emergent_candidate_signals,
    );
    render_signal_section(
        &mut out,
        "Unknown Structure Candidates",
        &report.unknown_structure_candidates,
    );
    render_cluster_section(&mut out, "Repeated Anomaly Clusters", &report.repeated_anomaly_clusters);
    render_cluster_section(
        &mut out,
        "Category Mismatch Clusters",
        &report.category_mismatch_clusters,
    );
    out.push_str("## Cross-Session Summary\n");
    for summary in &report.cross_session_summary {
        out.push_str(&format!("- {summary}\n"));
    }
    out.push('\n');
    out.push_str("## Caution Notes\n");
    for note in &report.caution_notes {
        out.push_str(&format!("- {note}\n"));
    }
    out.push_str("\n## Recommended Next Probes\n");
    for probe in &report.recommended_next_probes {
        out.push_str(&format!("- {probe}\n"));
    }
    out
}

fn render_signal_section(out: &mut String, title: &str, signals: &[Signal]) {
    out.push_str(&format!("## {title}\n"));
    if signals.is_empty() {
        out.push_str("- None flagged in this run.\n\n");
        return;
    }
    for signal in signals {
        out.push_str(&format!(
            "- {} ({})\n",
            signal.summary, signal.confidence
        ));
    }
    out.push('\n');
}

fn render_cluster_section(out: &mut String, title: &str, clusters: &[Cluster]) {
    out.push_str(&format!("## {title}\n"));
    if clusters.is_empty() {
        out.push_str("- None clustered in this run.\n\n");
        return;
    }
    for cluster in clusters {
        out.push_str(&format!(
            "- {} items in {}: {}\n",
            cluster.count, cluster.cluster_type, cluster.interpretation
        ));
    }
    out.push('\n');
}

fn build_anomaly_clusters(interactions: &[InteractionEvent]) -> Vec<Cluster> {
    cluster_maps_from_flags(interactions)
        .into_iter()
        .filter(|(_, items)| items.len() >= 2)
        .map(|(flag, items)| Cluster {
            cluster_id: Uuid::new_v4().to_string(),
            cluster_type: flag.clone(),
            item_ids: items.iter().map(|event| event.item_id.clone()).collect(),
            supporting_event_ids: items.iter().map(|event| event.event_id.clone()).collect(),
            count: items.len() as u32,
            interpretation: format!("Repeated {} across similar items.", flag),
        })
        .collect()
}

fn build_mismatch_clusters(interactions: &[InteractionEvent]) -> Vec<Cluster> {
    let mut grouped: HashMap<String, Vec<&InteractionEvent>> = HashMap::new();
    for event in interactions.iter().filter(|event| event.structure_fit == "mismatch") {
        grouped.entry(event.domain_id.clone()).or_default().push(event);
    }

    grouped
        .into_iter()
        .filter(|(_, items)| items.len() >= 2)
        .map(|(domain_id, items)| Cluster {
            cluster_id: Uuid::new_v4().to_string(),
            cluster_type: "category_mismatch".to_string(),
            item_ids: items.iter().map(|event| event.item_id.clone()).collect(),
            supporting_event_ids: items.iter().map(|event| event.event_id.clone()).collect(),
            count: items.len() as u32,
            interpretation: format!(
                "Repeated mismatch in domain {} suggests label-to-skill misalignment.",
                domain_id
            ),
        })
        .collect()
}

fn build_caution_notes(
    interactions: &[InteractionEvent],
    refusals: &[RefusalEvent],
    anomalies: &[AnomalyEvent],
) -> Vec<String> {
    vec![
        format!("Total interactions analyzed: {}.", interactions.len()),
        format!("Refusal events recorded: {}.", refusals.len()),
        format!("Anomaly events recorded: {}.", anomalies.len()),
        "This analyzer is rule-based and intentionally conservative.".to_string(),
    ]
}

fn build_next_probes(interactions: &[InteractionEvent], refusals: &[RefusalEvent]) -> Vec<String> {
    let mut probes = Vec::new();
    if refusals.len() >= 3 {
        probes.push(
            "Add near-transfer items that differ only slightly from approved exact-memory prompts."
                .to_string(),
        );
    }
    if interactions
        .iter()
        .any(|event| event.anomaly_flags.iter().any(|flag| flag == "boundary_pressure"))
    {
        probes.push(
            "Add adjacent-skill ordering probes with reduced ambiguity to test boundary pressure."
                .to_string(),
        );
    }
    if probes.is_empty() {
        probes.push("Repeat the smoke run with a slightly broader deterministic skill manifest.".to_string());
    }
    probes
}

fn build_cross_session_summary(
    current_run_id: &str,
    confirmed_memory_count: usize,
    confirmed_skill_count: usize,
    confirmed_signals: &[Signal],
    boundary_signals: &[Signal],
    emergent_candidate_signals: &[Signal],
    prior_sessions: &[CrossSessionInput],
) -> Vec<String> {
    if prior_sessions.is_empty() {
        return vec![format!(
            "No prior completed sessions were available for aggregation; this run recorded {} memory-backed confirmations and {} skill-backed confirmations.",
            confirmed_memory_count, confirmed_skill_count
        )];
    }

    let prior_run_count = prior_sessions.len();
    let total_runs = prior_run_count + 1;
    let recurring_boundary_runs = prior_sessions
        .iter()
        .filter(|session| session.boundary_count > 0)
        .count()
        + usize::from(!boundary_signals.is_empty());
    let recurring_emergent_runs = prior_sessions
        .iter()
        .filter(|session| session.emergent_count > 0)
        .count()
        + usize::from(!emergent_candidate_signals.is_empty());
    let recurring_anomaly_runs = prior_sessions
        .iter()
        .filter(|session| session.anomaly_count > 0)
        .count();
    let latest_runs = prior_sessions
        .iter()
        .rev()
        .take(3)
        .map(|session| session.run_id[..8.min(session.run_id.len())].to_string())
        .collect::<Vec<_>>();
    let dominant_backend = dominant_backend(prior_sessions);
    let average_refusals = if prior_run_count == 0 {
        0.0
    } else {
        prior_sessions
            .iter()
            .map(|session| session.refusal_count as f64)
            .sum::<f64>()
            / prior_run_count as f64
    };
    let current_confirmed = confirmed_signals.len();

    vec![
        format!(
            "Cross-session view covers {} runs including current run {}.",
            total_runs,
            &current_run_id[..8.min(current_run_id.len())]
        ),
        format!(
            "Boundary signals appeared in {} of {} runs; emergent candidate signals appeared in {} of {} runs.",
            recurring_boundary_runs, total_runs, recurring_emergent_runs, total_runs
        ),
        format!(
            "Prior completed runs averaged {:.1} refusals; the current run recorded {} confirmed signals with {} memory-backed and {} skill-backed confirmations.",
            average_refusals, current_confirmed, confirmed_memory_count, confirmed_skill_count
        ),
        format!(
            "Anomaly-bearing prior runs: {}. Dominant prior teacher backend: {}.",
            recurring_anomaly_runs,
            dominant_backend
        ),
        if latest_runs.is_empty() {
            "No prior run ids were available for drift comparison.".to_string()
        } else {
            format!(
                "Most recent prior runs considered for drift comparison: {}.",
                latest_runs.join(", ")
            )
        },
    ]
}

fn dominant_backend(prior_sessions: &[CrossSessionInput]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for session in prior_sessions {
        *counts.entry(session.teacher_backend_id.clone()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(backend, _)| backend)
        .unwrap_or_else(|| "n/a".to_string())
}

fn confirmed_signal_type(
    event: &InteractionEvent,
    skill_by_item: &HashMap<&str, &SkillEvent>,
    memory_read_items: &HashMap<&str, usize>,
) -> String {
    if memory_read_items.contains_key(event.item_id.as_str()) {
        "confirmed_memory_signal".to_string()
    } else if skill_by_item
        .get(event.item_id.as_str())
        .and_then(|skill| skill.executed_skill.as_ref())
        .is_some()
    {
        "confirmed_skill_signal".to_string()
    } else {
        "confirmed_signal".to_string()
    }
}

fn confirmed_signal_summary(
    event: &InteractionEvent,
    skill_by_item: &HashMap<&str, &SkillEvent>,
    memory_read_items: &HashMap<&str, usize>,
) -> String {
    if let Some(skill) = skill_by_item
        .get(event.item_id.as_str())
        .and_then(|skill| skill.executed_skill.as_deref())
        && !memory_read_items.contains_key(event.item_id.as_str())
    {
        return format!("{} matched expected structure via skill {}.", event.item_id, skill);
    }
    if memory_read_items.contains_key(event.item_id.as_str()) {
        format!("{} matched expected structure via explicit memory.", event.item_id)
    } else {
        format!("{} matched expected structure.", event.item_id)
    }
}

fn confirmed_signal_explanation(
    event: &InteractionEvent,
    skill_by_item: &HashMap<&str, &SkillEvent>,
    memory_read_items: &HashMap<&str, usize>,
) -> String {
    if memory_read_items.contains_key(event.item_id.as_str()) {
        "Correct answer with expected alignment and explicit memory read.".to_string()
    } else if let Some(skill) = skill_by_item
        .get(event.item_id.as_str())
        .and_then(|skill| skill.executed_skill.as_deref())
    {
        format!("Correct answer with expected skill execution: {}.", skill)
    } else {
        "Correct answer with expected structure alignment.".to_string()
    }
}

fn cluster_maps_from_flags(interactions: &[InteractionEvent]) -> HashMap<String, Vec<&InteractionEvent>> {
    let mut grouped: HashMap<String, Vec<&InteractionEvent>> = HashMap::new();
    for event in interactions {
        for flag in &event.anomaly_flags {
            grouped.entry(flag.clone()).or_default().push(event);
        }
    }
    grouped
}

fn cluster_by_flag(interactions: &[InteractionEvent], flag: &str) -> Vec<Cluster> {
    let grouped = cluster_maps_from_flags(interactions);
    grouped
        .get(flag)
        .map(|items| {
            vec![Cluster {
                cluster_id: Uuid::new_v4().to_string(),
                cluster_type: flag.to_string(),
                item_ids: items.iter().map(|event| event.item_id.clone()).collect(),
                supporting_event_ids: items.iter().map(|event| event.event_id.clone()).collect(),
                count: items.len() as u32,
                interpretation: format!("Repeated {} events.", flag),
            }]
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryEvent;
    use crate::skills::SkillEvent;
    use crate::telemetry::{AnomalyEvent, InteractionEvent, RefusalEvent};

    #[test]
    fn report_classifies_boundary_and_emergent_signals_conservatively() {
        let interactions = vec![
            interaction("evt-1", "item-1", "attention_discrimination", "correct", "matched", vec![]),
            interaction("evt-2", "item-2", "rule_exception", "refused", "mismatch", vec!["unexpected_refusal"]),
            interaction("evt-3", "item-3", "rule_exception", "refused", "mismatch", vec!["unexpected_refusal"]),
            interaction("evt-4", "item-4", "rule_exception", "refused", "mismatch", vec!["unexpected_refusal", "boundary_pressure"]),
        ];
        let refusals = vec![
            refusal("item-2"),
            refusal("item-3"),
            refusal("item-4"),
        ];
        let anomalies = vec![
            anomaly("item-4", vec!["boundary_pressure", "structure_mismatch"]),
        ];
        let skill_events = vec![
            skill("item-1", Some("option_match_selector")),
        ];
        let memory_events = vec![memory("item-1")];

        let report = build_report(
            "session-a",
            "run-a",
            &interactions,
            &refusals,
            &anomalies,
            &skill_events,
            &memory_events,
            &[CrossSessionInput {
                run_id: "run-prior".to_string(),
                run_mode: "smoke".to_string(),
                teacher_backend_id: "mock".to_string(),
                total_items: 30,
                refusal_count: 2,
                anomaly_count: 1,
                confirmed_count: 3,
                boundary_count: 1,
                emergent_count: 0,
            }],
        );

        assert_eq!(report.boundary_signals.len(), 1);
        assert_eq!(report.emergent_candidate_signals.len(), 1);
        assert!(
            report
                .recommended_next_probes
                .iter()
                .any(|probe| probe.contains("near-transfer"))
        );
        assert!(
            report
                .recommended_next_probes
                .iter()
                .any(|probe| probe.contains("boundary pressure"))
        );
    }

    #[test]
    fn markdown_render_includes_required_sections() {
        let report = build_report(
            "session-b",
            "run-b",
            &[interaction(
                "evt-10",
                "item-10",
                "attention_discrimination",
                "correct",
                "matched",
                vec![],
            )],
            &[],
            &[],
            &[skill("item-10", Some("option_match_selector"))],
            &[],
            &[],
        );

        let markdown = render_markdown(&report);

        assert!(markdown.contains("## Confirmed Signals"));
        assert!(markdown.contains("## Boundary Signals"));
        assert!(markdown.contains("## Emergent Candidate Signals"));
        assert!(markdown.contains("## Unknown Structure Candidates"));
        assert!(markdown.contains("## Repeated Anomaly Clusters"));
        assert!(markdown.contains("## Category Mismatch Clusters"));
        assert!(markdown.contains("## Cross-Session Summary"));
        assert!(markdown.contains("## Caution Notes"));
        assert!(markdown.contains("## Recommended Next Probes"));
    }

    #[test]
    fn report_clusters_repeated_anomalies_and_category_mismatches() {
        let interactions = vec![
            interaction(
                "evt-20",
                "item-20",
                "rule_exception",
                "incorrect",
                "mismatch",
                vec!["unexpected_failure", "structure_mismatch"],
            ),
            interaction(
                "evt-21",
                "item-21",
                "rule_exception",
                "incorrect",
                "mismatch",
                vec!["unexpected_failure", "structure_mismatch"],
            ),
            interaction(
                "evt-22",
                "item-22",
                "rule_exception",
                "incorrect",
                "mismatch",
                vec!["unexpected_failure"],
            ),
        ];

        let report = build_report(
            "session-c",
            "run-c",
            &interactions,
            &[],
            &[],
            &[],
            &[],
            &[],
        );

        assert!(
            report
                .repeated_anomaly_clusters
                .iter()
                .any(|cluster| cluster.cluster_type == "unexpected_failure" && cluster.count >= 2)
        );
        assert!(
            report
                .category_mismatch_clusters
                .iter()
                .any(|cluster| cluster.cluster_type == "category_mismatch" && cluster.count >= 2)
        );
    }

    #[test]
    fn emergent_candidates_remain_provisional_and_require_review() {
        let interactions = vec![
            interaction("evt-30", "item-30", "abstraction_transfer", "refused", "mismatch", vec!["unexpected_refusal"]),
            interaction("evt-31", "item-31", "abstraction_transfer", "refused", "mismatch", vec!["unexpected_refusal"]),
            interaction("evt-32", "item-32", "abstraction_transfer", "refused", "mismatch", vec!["unexpected_refusal"]),
        ];
        let refusals = vec![refusal("item-30"), refusal("item-31"), refusal("item-32")];

        let report = build_report(
            "session-d",
            "run-d",
            &interactions,
            &refusals,
            &[],
            &[],
            &[],
            &[],
        );

        assert_eq!(report.emergent_candidate_signals.len(), 1);
        let emergent = &report.emergent_candidate_signals[0];
        assert_eq!(emergent.signal_type, "emergent_candidate_signal");
        assert!(emergent.requires_human_review);
        assert_eq!(emergent.confidence, "low");
        assert!(emergent.summary.to_ascii_lowercase().contains("cluster"));
        assert!(!emergent.summary.to_ascii_lowercase().contains("proven"));
        assert!(!emergent.explanation.to_ascii_lowercase().contains("intelligence"));
    }

    fn interaction(
        event_id: &str,
        item_id: &str,
        domain_id: &str,
        correctness: &str,
        structure_fit: &str,
        anomaly_flags: Vec<&str>,
    ) -> InteractionEvent {
        InteractionEvent {
            event_id: event_id.to_string(),
            session_id: "session".to_string(),
            run_id: "run".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            curriculum_version: "0.1.0".to_string(),
            item_id: item_id.to_string(),
            domain_id: domain_id.to_string(),
            concept_id: format!("{domain_id}_concept"),
            item_type: "probe_boundary".to_string(),
            teacher_backend_id: "mock".to_string(),
            teacher_prompt: None,
            generated_question: "question".to_string(),
            intended_structure: vec!["structure".to_string()],
            expected_answer: Some("answer".to_string()),
            janet_answer: None,
            correctness_judgment: correctness.to_string(),
            teacher_feedback: None,
            structure_fit: structure_fit.to_string(),
            anomaly_flags: anomaly_flags.into_iter().map(str::to_string).collect(),
            latency_ms: 1,
            raw_event_trace_hash: None,
            code_version_hash: Some("0.1.0".to_string()),
        }
    }

    fn refusal(item_id: &str) -> RefusalEvent {
        RefusalEvent {
            refusal_event_id: format!("refusal-{item_id}"),
            session_id: "session".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            item_id: item_id.to_string(),
            reason: "blocked".to_string(),
            uncertainty_state: "high".to_string(),
            candidate_next_steps: vec!["review curriculum item".to_string()],
            policy_trace: vec!["deterministic_only=true".to_string()],
        }
    }

    fn anomaly(item_id: &str, flags: Vec<&str>) -> AnomalyEvent {
        AnomalyEvent {
            anomaly_event_id: format!("anomaly-{item_id}"),
            session_id: "session".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            item_id: item_id.to_string(),
            structure_fit: "partial_match".to_string(),
            anomaly_flags: flags.into_iter().map(str::to_string).collect(),
            observed_structure: vec!["structure".to_string()],
            structure_fit_explanation: "explanation".to_string(),
            supporting_trace_ids: vec![format!("trace-{item_id}")],
        }
    }

    fn skill(item_id: &str, executed_skill: Option<&str>) -> SkillEvent {
        SkillEvent {
            skill_event_id: format!("skill-{item_id}"),
            session_id: "session".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            item_id: item_id.to_string(),
            candidate_skills: vec!["option_match_selector".to_string()],
            approved_skills: vec!["option_match_selector".to_string()],
            blocked_skills: Vec::new(),
            executed_skill: executed_skill.map(str::to_string),
            approval_ledger_version: "test".to_string(),
            reason: "ok".to_string(),
        }
    }

    fn memory(item_id: &str) -> MemoryEvent {
        MemoryEvent {
            memory_event_id: format!("memory-{item_id}"),
            session_id: "session".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            operation: "read".to_string(),
            memory_scope: "explicit_exact_answers".to_string(),
            key: "key".to_string(),
            value_before: None,
            value_after: None,
            reason: "lookup".to_string(),
            source_item_id: Some(item_id.to_string()),
            approved_by_policy: true,
        }
    }
}

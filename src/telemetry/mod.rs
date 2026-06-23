use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InteractionEvent {
    pub event_id: String,
    pub session_id: String,
    pub run_id: String,
    pub timestamp: String,
    pub curriculum_version: String,
    pub item_id: String,
    pub domain_id: String,
    pub concept_id: String,
    pub item_type: String,
    pub teacher_backend_id: String,
    pub teacher_prompt: Option<String>,
    pub generated_question: String,
    pub intended_structure: Vec<String>,
    pub expected_answer: Option<String>,
    pub janet_answer: Option<String>,
    pub correctness_judgment: String,
    pub teacher_feedback: Option<String>,
    pub structure_fit: String,
    pub anomaly_flags: Vec<String>,
    pub latency_ms: u64,
    pub raw_event_trace_hash: Option<String>,
    pub code_version_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefusalEvent {
    pub refusal_event_id: String,
    pub session_id: String,
    pub timestamp: String,
    pub item_id: String,
    pub reason: String,
    pub uncertainty_state: String,
    pub candidate_next_steps: Vec<String>,
    pub policy_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnomalyEvent {
    pub anomaly_event_id: String,
    pub session_id: String,
    pub timestamp: String,
    pub item_id: String,
    pub structure_fit: String,
    pub anomaly_flags: Vec<String>,
    pub observed_structure: Vec<String>,
    pub structure_fit_explanation: String,
    pub supporting_trace_ids: Vec<String>,
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillEvent {
    pub skill_event_id: String,
    pub session_id: String,
    pub timestamp: String,
    pub item_id: String,
    pub candidate_skills: Vec<String>,
    pub approved_skills: Vec<String>,
    pub blocked_skills: Vec<String>,
    pub executed_skill: Option<String>,
    pub approval_ledger_version: String,
    pub reason: String,
}

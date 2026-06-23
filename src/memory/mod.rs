use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemoryEvent {
    pub memory_event_id: String,
    pub session_id: String,
    pub timestamp: String,
    pub operation: String,
    pub memory_scope: String,
    pub key: String,
    pub value_before: Option<serde_json::Value>,
    pub value_after: Option<serde_json::Value>,
    pub reason: String,
    pub source_item_id: Option<String>,
    pub approved_by_policy: bool,
}

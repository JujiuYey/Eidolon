use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConversation {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub agent_profile_id: String,

    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default = "default_snapshot_version")]
    pub snapshot_version: u32,

    #[serde(default)]
    pub created_from_profile_updated_at: i64,

    #[serde(default)]
    pub snapshot_agent_name: String,

    #[serde(default)]
    pub snapshot_provider_id: String,

    #[serde(default)]
    pub snapshot_model_id: String,

    #[serde(default = "default_temperature")]
    pub snapshot_temperature: String,

    #[serde(default = "default_max_tokens")]
    pub snapshot_max_tokens: String,

    #[serde(default)]
    pub snapshot_system_prompt: String,

    #[serde(default)]
    pub snapshot_enabled_mcp_service_ids: Vec<String>,

    #[serde(default)]
    pub snapshot_enabled_tool_keys: Vec<String>,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConversationMessage {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub conversation_id: String,

    #[serde(default)]
    pub role: String,

    #[serde(default)]
    pub content: String,

    #[serde(default = "default_status")]
    pub status: String,

    #[serde(default)]
    pub created_at: i64,
}

fn default_title() -> String {
    "新对话".to_string()
}

fn default_snapshot_version() -> u32 {
    1
}

fn default_temperature() -> String {
    "0.7".to_string()
}

fn default_max_tokens() -> String {
    "4096".to_string()
}

fn default_status() -> String {
    "done".to_string()
}

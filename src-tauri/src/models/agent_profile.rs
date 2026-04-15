use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfile {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub provider_id: String,

    #[serde(default)]
    pub model_id: String,

    #[serde(default = "default_temperature")]
    pub temperature: String,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: String,

    #[serde(default)]
    pub system_prompt: String,

    #[serde(default)]
    pub work_directory: String,

    #[serde(default)]
    pub enabled_mcp_service_ids: Vec<String>,

    #[serde(default)]
    pub enabled_tool_keys: Vec<String>,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

fn default_temperature() -> String {
    "0.7".to_string()
}

fn default_max_tokens() -> String {
    "4096".to_string()
}

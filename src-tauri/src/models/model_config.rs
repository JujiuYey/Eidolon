use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSetting {
    pub provider_id: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub api_key: String,

    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelCapabilities {
    #[serde(default)]
    pub chat: bool,

    #[serde(default)]
    pub vision: bool,

    #[serde(default)]
    pub tool_call: bool,

    #[serde(default)]
    pub reasoning: bool,

    #[serde(default)]
    pub embedding: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModel {
    pub provider_id: String,

    pub model_id: String,

    #[serde(default)]
    pub capabilities: ProviderModelCapabilities,
}

fn default_true() -> bool {
    true
}

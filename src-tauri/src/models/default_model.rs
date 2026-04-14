use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultModelSetting {
    pub key: String,
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub temperature: String,
    #[serde(default)]
    pub top_p: String,
    #[serde(default)]
    pub max_tokens: String,
    #[serde(default)]
    pub presence_penalty: String,
    #[serde(default)]
    pub frequency_penalty: String,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 单个平台的配置文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 平台唯一标识，内置平台使用固定字符串（"minimax" 等），
    /// 用户自定义平台可使用任意非空字符串。
    pub provider_id: String,

    /// 该平台在当前应用中是否启用。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// API 密钥，Ollama 等无需 key 的平台留空即可。
    #[serde(default)]
    pub api_key: String,

    /// 覆盖默认 base_url，留空时由前端 registry 提供默认值。
    #[serde(default)]
    pub base_url: String,

    /// 文档创建时间。
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// 文档最后更新时间。
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_true() -> bool {
    true
}

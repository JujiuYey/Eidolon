use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 固定的平台标识。
/// 建议直接作为 MongoDB 文档的 `_id` 使用，保证每个平台只有一条配置文档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKey {
    /// MiniMax 平台。
    Minimax,
    /// 火山引擎平台。
    Volcengine,
    /// Ollama 平台。
    Ollama,
    /// DeepSeek 平台。
    Deepseek,
}

impl ProviderKey {
    /// 返回稳定的字符串键，便于作为 MongoDB `_id` 和查询条件使用。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimax => "minimax",
            Self::Volcengine => "volcengine",
            Self::Ollama => "ollama",
            Self::Deepseek => "deepseek",
        }
    }
}

/// 平台连接配置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConnection {
    /// 平台 API 密钥。
    #[serde(default)]
    pub api_key: Option<String>,

    /// 平台的基础请求地址。
    #[serde(default)]
    pub base_url: Option<String>,
}

/// 单个模型支持的能力标签。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// 是否支持普通文本对话。
    #[serde(default)]
    pub chat: bool,

    /// 是否支持视觉输入。
    #[serde(default)]
    pub vision: bool,

    /// 是否支持工具调用。
    #[serde(default)]
    pub tool_call: bool,

    /// 是否支持推理型输出。
    #[serde(default)]
    pub reasoning: bool,

    /// 是否支持 embedding。
    #[serde(default)]
    pub embedding: bool,
}

/// 平台下的单个模型定义。
/// 这是模型列表中的一项，而不是平台级配置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModel {
    /// 模型的唯一标识。
    pub id: String,

    /// 模型展示名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 当前应用中该模型是否启用。
    #[serde(default)]
    pub enabled: bool,

    /// 该模型具备的能力集合。
    #[serde(default)]
    pub capabilities: ModelCapabilities,

    /// 模型支持的上下文窗口大小。
    #[serde(default)]
    pub context_window: Option<i32>,

    /// 模型的最大输出 token 数。
    #[serde(default)]
    pub max_output_tokens: Option<i32>,
}

/// 平台的模型目录。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCatalog {
    /// 当前平台下可用的模型列表。
    #[serde(default)]
    pub items: Vec<ProviderModel>,
}

/// 单个平台的配置文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderSettingDoc {
    /// 文档主键。
    /// 直接复用 `ProviderKey`，避免再引入随机 id。
    #[serde(rename = "_id")]
    pub provider_key: ProviderKey,

    /// 该平台在当前应用中是否启用。
    #[serde(default)]
    pub enabled: bool,

    /// 平台连接相关配置。
    #[serde(default)]
    pub connection: ProviderConnection,

    /// 当前选中的模型 id。
    /// 只存引用，不重复存整份模型对象。
    #[serde(default)]
    pub selected_model_id: Option<String>,

    /// 平台下的模型目录。
    #[serde(default)]
    pub catalog: ModelCatalog,

    /// 文档创建时间。
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// 文档最后更新时间。
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

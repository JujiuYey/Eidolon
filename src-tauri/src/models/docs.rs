use serde::{Deserialize, Serialize};

/// 文档管理设置（docs_settings.json）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsSettings {
    /// 文档库根目录。文档是磁盘上的普通 .md 文件，不进 SQLite
    #[serde(default)]
    pub root_dir: Option<String>,
}

/// 文档库条目（文件或文件夹），path 为相对根目录的 `/` 分隔路径
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsEntry {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub path: String,

    #[serde(default)]
    pub is_dir: bool,

    /// 文件字节数，文件夹为 0
    #[serde(default)]
    pub size: u64,

    /// 修改时间（毫秒），取自文件系统
    #[serde(default)]
    pub updated_at: i64,
}

/// 全文搜索命中行
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsSearchResult {
    #[serde(default)]
    pub path: String,

    #[serde(default)]
    pub name: String,

    /// 从 1 开始的行号
    #[serde(default)]
    pub line_number: u32,

    #[serde(default)]
    pub line_text: String,
}

/// 文档 AI 助手的一条会话消息（仅前端内存态，不落库）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsAiChatMessage {
    #[serde(default)]
    pub role: String,

    #[serde(default)]
    pub content: String,
}

/// 一轮文档助手回答
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsAiChatResult {
    #[serde(default)]
    pub content: String,

    /// 实际使用的模型标识（provider/model）
    #[serde(default)]
    pub model_label: String,
}

use serde::{Deserialize, Serialize};

/// 产品经理分身的会话
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PmConversation {
    #[serde(default)]
    pub id: String,

    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// 会话内的一条消息。发送失败的消息不会入库，因此没有 error 状态
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PmMessage {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub conversation_id: String,

    #[serde(default)]
    pub role: String,

    #[serde(default)]
    pub content: String,

    /// 本轮回答实际参考的方法论框架（slug 列表），由路由阶段决定
    #[serde(default)]
    pub skills_used: Vec<String>,

    #[serde(default)]
    pub created_at: i64,
}

/// 方法论框架清单条目（设置页展示用）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PmSkillSummary {
    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,

    /// "builtin"（随应用打包）或 "override"（来自覆盖目录）
    #[serde(default)]
    pub source: String,

    #[serde(default)]
    pub size: u64,
}

/// 产品经理分身的功能设置
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PmSettings {
    /// 覆盖目录（可选）。目录下每个子文件夹的 SKILL.md 按文件夹名覆盖内置版本
    #[serde(default)]
    pub skills_override_dir: Option<String>,
}

fn default_title() -> String {
    "新对话".to_string()
}

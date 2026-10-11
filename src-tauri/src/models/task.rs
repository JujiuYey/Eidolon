//! 个人任务 DTO。
//!
//! 轻量任务清单（对应 `tasks` 表）：标题 + 备注 + 状态 + 优先级 + 截止日期。
//! `source` 预留外部来源（如禅道引用），当前恒为 manual。

use serde::{Deserialize, Serialize};

/// 任务 AI 功能使用的默认模型 key
pub const TASK_ASSISTANT_MODEL_KEY: &str = "task_assistant";

pub const TASK_STATUS_TODO: &str = "todo";
pub const TASK_STATUS_DOING: &str = "doing";
pub const TASK_STATUS_DONE: &str = "done";

pub const TASK_PRIORITY_HIGH: &str = "high";
pub const TASK_PRIORITY_MEDIUM: &str = "medium";
pub const TASK_PRIORITY_LOW: &str = "low";
pub const TASK_PRIORITY_NONE: &str = "none";

/// 任务（对应 `tasks` 表）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    /// 任务标题
    #[serde(default)]
    pub title: String,
    /// 补充备注（纯文本，可多行）
    #[serde(default)]
    pub notes: String,
    /// todo | doing | done
    #[serde(default = "default_status")]
    pub status: String,
    /// high | medium | low | none
    #[serde(default = "default_priority")]
    pub priority: String,
    /// 截止日期 YYYY-MM-DD；空字符串表示无截止
    #[serde(default)]
    pub due_date: String,
    /// 来源：manual（预留外部来源）
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub sort: i64,
    /// 完成时间，Unix 毫秒；未完成为 None
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// AI 从一句话解析出的任务草稿（不含状态/备注，落库前由前端补齐）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDraft {
    #[serde(default)]
    pub title: String,
    /// 已归一化为 high | medium | low | none
    #[serde(default = "default_priority")]
    pub priority: String,
    /// YYYY-MM-DD；无法识别时为空字符串
    #[serde(default)]
    pub due_date: String,
}

/// 归一化状态：非法值回落为 todo
pub fn normalize_status(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        TASK_STATUS_DOING => TASK_STATUS_DOING.to_string(),
        TASK_STATUS_DONE => TASK_STATUS_DONE.to_string(),
        _ => TASK_STATUS_TODO.to_string(),
    }
}

/// 归一化优先级：非法值回落为 none
pub fn normalize_priority(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        TASK_PRIORITY_HIGH => TASK_PRIORITY_HIGH.to_string(),
        TASK_PRIORITY_MEDIUM => TASK_PRIORITY_MEDIUM.to_string(),
        TASK_PRIORITY_LOW => TASK_PRIORITY_LOW.to_string(),
        _ => TASK_PRIORITY_NONE.to_string(),
    }
}

/// 校验 YYYY-MM-DD 日期；合法原样返回（去空白），否则 None
pub fn normalize_due_date(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .map(|_| trimmed.to_string())
}

fn default_source() -> String {
    "manual".to_string()
}

fn default_status() -> String {
    TASK_STATUS_TODO.to_string()
}

fn default_priority() -> String {
    TASK_PRIORITY_NONE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_deserializes_from_partial_json() {
        let task: Task = serde_json::from_str(r#"{"title": "写周报"}"#).expect("partial json");
        assert_eq!(task.title, "写周报");
        assert_eq!(task.status, "todo", "status should default to todo");
        assert_eq!(task.priority, "none", "priority should default to none");
        assert_eq!(task.source, "manual");
        assert_eq!(task.completed_at, None);
        assert!(task.due_date.is_empty());
    }

    #[test]
    fn normalize_status_falls_back_to_todo() {
        assert_eq!(normalize_status("doing"), "doing");
        assert_eq!(normalize_status(" done "), "done");
        assert_eq!(normalize_status("archived"), "todo");
        assert_eq!(normalize_status(""), "todo");
    }

    #[test]
    fn normalize_priority_falls_back_to_none() {
        assert_eq!(normalize_priority("high"), "high");
        assert_eq!(normalize_priority(" LOW "), "low");
        assert_eq!(normalize_priority("urgent"), "none");
    }

    #[test]
    fn normalize_due_date_accepts_only_iso_dates() {
        assert_eq!(normalize_due_date("2026-10-15").as_deref(), Some("2026-10-15"));
        assert_eq!(normalize_due_date(" 2026-10-15 ").as_deref(), Some("2026-10-15"));
        assert_eq!(normalize_due_date(""), None);
        assert_eq!(normalize_due_date("周五"), None);
        assert_eq!(normalize_due_date("2026-13-40"), None);
    }
}

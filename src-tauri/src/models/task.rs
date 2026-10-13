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
    /// 排期开始，本地无时区 ISO（YYYY-MM-DDTHH:MM）；空字符串表示未排期
    #[serde(default)]
    pub start_at: String,
    /// 排期结束，本地无时区 ISO（YYYY-MM-DDTHH:MM）；与 start_at 成对出现
    #[serde(default)]
    pub end_at: String,
    /// 预估时长（分钟），拖入日历的默认块大小与 AI 排期参考；0 = 未设置
    #[serde(default)]
    pub estimate_minutes: i64,
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

/// 校验本地无时区日期时间（YYYY-MM-DDTHH:MM）；合法原样返回（去空白），否则 None
pub fn normalize_local_datetime(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M")
        .ok()
        .map(|_| trimmed.to_string())
}

/// 排期成对归一化：
/// - 两者都合法且 end > start 才保留
/// - 跨日块裁到开始日的 23:59（单块不跨日）
/// - 任一不合法或时序颠倒时双双清空（宽松策略，对齐 due_date 的容错）
pub fn normalize_schedule(start_at: &str, end_at: &str) -> (String, String) {
    let (Some(start), Some(end)) = (normalize_local_datetime(start_at), normalize_local_datetime(end_at))
    else {
        return (String::new(), String::new());
    };

    let start_dt = chrono::NaiveDateTime::parse_from_str(&start, "%Y-%m-%dT%H:%M").expect("revalidated");
    let end_dt = chrono::NaiveDateTime::parse_from_str(&end, "%Y-%m-%dT%H:%M").expect("revalidated");

    if end_dt <= start_dt {
        return (String::new(), String::new());
    }

    // 跨日：裁到开始日 23:59，保证单块不跨日
    let clamped_end = if end_dt.date() > start_dt.date() {
        format!("{}T23:59", start_dt.date().format("%Y-%m-%d"))
    } else {
        end
    };

    (start, clamped_end)
}

/// 预估时长归一化：负数归零，封顶一天
pub fn normalize_estimate_minutes(value: i64) -> i64 {
    value.clamp(0, 24 * 60)
}

/// AI 排期建议中的单个时间块（已映射回任务实体，供前端展示）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanBlock {
    pub task_id: String,
    /// 任务标题（取自任务本体，非 AI 输出）
    #[serde(default)]
    pub title: String,
    /// 任务优先级（取自任务本体）
    #[serde(default)]
    pub priority: String,
    pub start_at: String,
    pub end_at: String,
}

/// AI 排期建议：结构化时间块 + 一句安排思路
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlan {
    pub blocks: Vec<TaskPlanBlock>,
    #[serde(default)]
    pub note: String,
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

    #[test]
    fn normalize_schedule_keeps_valid_pairs() {
        let (start, end) = normalize_schedule("2026-10-14T09:00", "2026-10-14T10:30");
        assert_eq!(start, "2026-10-14T09:00");
        assert_eq!(end, "2026-10-14T10:30");
    }

    #[test]
    fn normalize_schedule_clears_broken_pairs() {
        let empty = (String::new(), String::new());
        assert_eq!(normalize_schedule("", "2026-10-14T10:00"), empty);
        assert_eq!(normalize_schedule("2026-10-14T09:00", ""), empty);
        assert_eq!(normalize_schedule("周五上午", "2026-10-14T10:00"), empty);
        assert_eq!(normalize_schedule("2026-10-14T11:00", "2026-10-14T10:00"), empty);
    }

    #[test]
    fn normalize_schedule_clamps_cross_day_blocks() {
        let (start, end) = normalize_schedule("2026-10-14T23:00", "2026-10-15T01:00");
        assert_eq!(start, "2026-10-14T23:00");
        assert_eq!(end, "2026-10-14T23:59", "cross-day end clamps to same day");
    }

    #[test]
    fn normalize_estimate_minutes_clamps_range() {
        assert_eq!(normalize_estimate_minutes(-5), 0);
        assert_eq!(normalize_estimate_minutes(90), 90);
        assert_eq!(normalize_estimate_minutes(10_000), 24 * 60);
    }
}

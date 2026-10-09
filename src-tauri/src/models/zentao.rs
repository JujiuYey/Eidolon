//! 禅道集成数据结构。
//!
//! 两类类型：
//! - `ZentaoApi*`：禅道 REST API v1 的原始响应结构（camelCase，字段尽量宽松）
//! - 其余：返回前端的 DTO（snake_case 直映，同 `models/email.rs` 约定）
//!
//! 已知怪癖（2026-10 对禅道 22.4 实测）：
//! - 任务列表里 `assignedTo` 是对象，Bug 列表里是纯账号字符串，甚至可能是 null
//! - 数值字段偶尔以字符串形式下发（如 `"8"`）
//! - 列表接口忽略所有查询参数，过滤只能客户端做

use serde::{Deserialize, Serialize};

/// 视为"进行中"的任务状态，其余（done/closed/canceled）不展示
pub const ACTIVE_TASK_STATUSES: [&str; 3] = ["wait", "doing", "pause"];

/// 视为"未关闭"的 Bug 状态：assignedTo 是我且未关闭就算我的 Bug
pub const CLOSED_BUG_STATUS: &str = "closed";

/// 视为"活跃"的执行状态，已关闭执行不再拉取任务
pub const ACTIVE_EXECUTION_STATUSES: [&str; 2] = ["wait", "doing"];

// ===== 禅道 API 原始结构 =====

/// 人员字段：任务列表是 `{account, realname}` 对象，Bug 列表是纯账号字符串
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ZentaoApiUser {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub realname: String,
}

pub fn deserialize_zentao_user<'de, D>(deserializer: D) -> Result<ZentaoApiUser, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(account) => ZentaoApiUser {
            account,
            realname: String::new(),
        },
        serde_json::Value::Object(map) => ZentaoApiUser {
            account: map
                .get("account")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            realname: map
                .get("realname")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        _ => ZentaoApiUser::default(),
    })
}

/// 字符串字段兜底：deadline 等字段在真实数据里会下发 null
pub fn deserialize_nullable_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(text) => text,
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    })
}

/// 数值字段兜底：禅道偶尔把数字下发为字符串
pub fn deserialize_flexible_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(i64),
        Text(String),
    }

    Ok(match Raw::deserialize(deserializer) {
        Ok(Raw::Number(value)) => value,
        Ok(Raw::Text(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    })
}

pub fn deserialize_flexible_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(f64),
        Text(String),
    }

    Ok(match Raw::deserialize(deserializer) {
        Ok(Raw::Number(value)) => value,
        Ok(Raw::Text(text)) => text.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    })
}

/// 执行（迭代/阶段），`/api.php/v1/executions` 列表项
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZentaoApiExecution {
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub id: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub begin: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub end: String,
}

/// 任务列表项，`/api.php/v1/executions/{id}/tasks`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZentaoApiTask {
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub id: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub pri: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_zentao_user")]
    pub assigned_to: ZentaoApiUser,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub execution: i64,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub project: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub deadline: String,
    #[serde(default, deserialize_with = "deserialize_flexible_f64")]
    pub estimate: f64,
    #[serde(default, deserialize_with = "deserialize_flexible_f64")]
    pub left: f64,
    #[serde(default, deserialize_with = "deserialize_flexible_f64")]
    pub consumed: f64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub story_title: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string", rename = "type")]
    pub task_type: String,
}

/// Bug 列表项，`/api.php/v1/executions/{id}/bugs`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZentaoApiBug {
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub id: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub severity: i64,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub pri: i64,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_zentao_user")]
    pub assigned_to: ZentaoApiUser,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub resolution: String,
    #[serde(default, deserialize_with = "deserialize_zentao_user")]
    pub opened_by: ZentaoApiUser,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub opened_date: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub deadline: String,
    #[serde(default, deserialize_with = "deserialize_flexible_i64")]
    pub execution: i64,
}

// ===== 前端 DTO =====

/// 禅道账户（对应 `zentao_accounts` 表）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZentaoAccount {
    #[serde(default)]
    pub id: String,
    /// 账户显示名（如 "公司禅道"）
    #[serde(default)]
    pub name: String,
    /// 站点根地址，如 `http://192.168.10.209`，可带子路径
    #[serde(default)]
    pub base_url: String,
    /// 登录账号
    #[serde(default)]
    pub account: String,
    /// 登录密码；遵循项目"掩码而非加密"约定，明文存储在本地 SQLite
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sort: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

fn default_true() -> bool {
    true
}

/// 归属当前用户的任务（已按执行维度聚合）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ZentaoTask {
    pub id: i64,
    pub name: String,
    pub pri: i64,
    pub status: String,
    pub assigned_to: ZentaoUser,
    /// 所属执行名称，由服务层从执行列表回填
    pub execution: i64,
    pub execution_name: String,
    pub deadline: String,
    /// 预计工时
    pub estimate: f64,
    /// 剩余工时
    pub left: f64,
    /// 已消耗工时
    pub consumed: f64,
    /// 关联需求标题，可为空
    pub story_title: String,
    /// 任务类型（devel/test/design 等）
    pub task_type: String,
}

/// 归属当前用户的 Bug
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ZentaoBug {
    pub id: i64,
    pub title: String,
    pub severity: i64,
    pub pri: i64,
    pub status: String,
    pub assigned_to: ZentaoUser,
    pub resolution: String,
    pub opened_by: ZentaoUser,
    pub opened_date: String,
    pub deadline: String,
    pub execution: i64,
    pub execution_name: String,
}

/// 禅道用户（统一为对象形态暴露给前端）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ZentaoUser {
    pub account: String,
    pub realname: String,
}

impl ZentaoUser {
    pub fn from_api(user: &ZentaoApiUser) -> Self {
        Self {
            account: user.account.clone(),
            realname: if user.realname.is_empty() {
                user.account.clone()
            } else {
                user.realname.clone()
            },
        }
    }
}

/// "我的工作台"聚合结果：当前账户名下的任务与 Bug
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ZentaoMyWork {
    pub account_id: String,
    pub tasks: Vec<ZentaoTask>,
    pub bugs: Vec<ZentaoBug>,
    /// 非致命告警（如个别执行拉取失败），不阻断整体结果
    pub warnings: Vec<String>,
    /// 拉取完成时间，Unix 毫秒
    pub fetched_at: i64,
}

// ===== 纯过滤逻辑（可单测）=====

/// 过滤出分配给 `account` 的进行中任务
pub fn filter_my_tasks<'a>(
    tasks: impl IntoIterator<Item = &'a ZentaoApiTask>,
    account: &str,
) -> Vec<ZentaoApiTask> {
    tasks
        .into_iter()
        .filter(|task| {
            task.assigned_to.account == account
                && ACTIVE_TASK_STATUSES.contains(&task.status.as_str())
        })
        .cloned()
        .collect()
}

/// 过滤出分配给 `account` 且未关闭的 Bug
pub fn filter_my_bugs<'a>(
    bugs: impl IntoIterator<Item = &'a ZentaoApiBug>,
    account: &str,
) -> Vec<ZentaoApiBug> {
    bugs.into_iter()
        .filter(|bug| {
            bug.assigned_to.account == account && bug.status != CLOSED_BUG_STATUS
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: i64, account: &str, status: &str) -> ZentaoApiTask {
        ZentaoApiTask {
            id,
            name: format!("任务{id}"),
            pri: 3,
            status: status.to_string(),
            assigned_to: ZentaoApiUser {
                account: account.to_string(),
                realname: String::new(),
            },
            execution: 7,
            project: 4,
            deadline: String::new(),
            estimate: 8.0,
            left: 4.0,
            consumed: 4.0,
            story_title: String::new(),
            task_type: "devel".to_string(),
        }
    }

    fn bug(id: i64, account: &str, status: &str) -> ZentaoApiBug {
        ZentaoApiBug {
            id,
            title: format!("Bug{id}"),
            severity: 2,
            pri: 2,
            status: status.to_string(),
            assigned_to: ZentaoApiUser {
                account: account.to_string(),
                realname: String::new(),
            },
            resolution: String::new(),
            opened_by: ZentaoApiUser::default(),
            opened_date: String::new(),
            deadline: String::new(),
            execution: 7,
        }
    }

    #[test]
    fn filter_tasks_keeps_only_active_mine() {
        let tasks = [
            task(1, "hjc", "wait"),
            task(2, "hjc", "doing"),
            task(3, "hjc", "pause"),
            task(4, "hjc", "closed"),
            task(5, "hjc", "done"),
            task(6, "hjc", "cancel"),
            task(7, "lwb", "doing"),
        ];

        let mine = filter_my_tasks(&tasks, "hjc");
        let ids: Vec<i64> = mine.iter().map(|item| item.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn filter_tasks_ignores_empty_account() {
        let mut unassigned = task(1, "wait", "wait");
        unassigned.assigned_to.account = String::new();

        assert!(filter_my_tasks([&unassigned], "hjc").is_empty());
    }

    #[test]
    fn filter_bugs_keeps_unclosed_mine() {
        let bugs = [
            bug(1, "hjc", "active"),
            bug(2, "hjc", "delay"),
            bug(3, "hjc", "closed"),
            bug(4, "lwb", "active"),
        ];

        let mine = filter_my_bugs(&bugs, "hjc");
        let ids: Vec<i64> = mine.iter().map(|item| item.id).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn zentao_user_serializes_as_object() {
        let user = ZentaoUser::from_api(&ZentaoApiUser {
            account: "hjc".to_string(),
            realname: String::new(),
        });
        assert_eq!(user.realname, "hjc", "空 realname 回退为账号");
    }
}

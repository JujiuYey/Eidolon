//! API Client 仓库之间的共享辅助：删除摘要、事务、键值行 JSON、
//! 行映射、跨表加载函数。所有 `pub(crate)` 成员仅供同模块的仓库使用。

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::models::api_client::{ApiEnvironment, ApiGroup, ApiProject, KeyValueRow};

/// 删除项目或分组时返回的关联数量，供前端在确认框中展示
///
/// 字段命名与前端 `TauriDeletionSummary` 完全一致（snake_case JSON）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeletionSummary {
    /// 被一并删除的分组数
    #[serde(default)]
    pub deleted_groups: i64,
    /// 被一并删除的请求数
    pub deleted_requests: i64,
    /// 被一并删除的历史条数
    pub deleted_histories: i64,
}

/// 共享的项目归属校验入口
pub(crate) fn require_project(
    transaction: &Transaction<'_>,
    project_id: &str,
) -> Result<(), String> {
    let exists: Option<i64> = transaction
        .query_row(
            "SELECT 1 FROM api_projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_error)?;

    if exists.is_none() {
        return Err(format!("未找到 id 为 {project_id} 的项目"));
    }

    Ok(())
}

/// 按分组 ID 查询所属项目
pub(crate) fn group_project_id(
    transaction: &Transaction<'_>,
    group_id: &str,
) -> Result<String, String> {
    transaction
        .query_row(
            "SELECT project_id FROM api_groups WHERE id = ?1",
            params![group_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {group_id} 的分组"))
}

/// 在事务中以 `MAX(sort) + 1` 方式生成下一个排序
pub(crate) fn next_sort<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    sql: &str,
    parameters: P,
) -> Result<i64, String> {
    transaction
        .query_row(sql, parameters, |row| row.get(0))
        .map_err(sql_error)
}

/// 启动 IMMEDIATE 事务，避免与级联写入产生竞争
pub(crate) fn begin(connection: &mut Connection) -> Result<Transaction<'_>, String> {
    connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("开始事务失败: {error}"))
}

pub(crate) fn sql_error(error: rusqlite::Error) -> String {
    format!("数据库操作失败: {error}")
}

pub(crate) fn require_name(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn require_timeout(timeout_ms: u64) -> Result<u64, String> {
    if timeout_ms == 0 {
        return Err("超时时间必须大于 0".to_string());
    }
    Ok(timeout_ms)
}

pub(crate) fn to_json(rows: &[KeyValueRow]) -> Result<String, String> {
    serde_json::to_string(rows).map_err(|error| format!("序列化键值行失败: {error}"))
}

pub(crate) fn from_json(value: &str) -> Result<Vec<KeyValueRow>, String> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value).map_err(|error| format!("解析键值行失败: {error}"))
}

pub(crate) fn map_project(row: &Row<'_>) -> rusqlite::Result<ApiProject> {
    Ok(ApiProject {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        sort: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub(crate) fn map_group(row: &Row<'_>) -> rusqlite::Result<ApiGroup> {
    Ok(ApiGroup {
        id: row.get(0)?,
        project_id: row.get(1)?,
        parent_group_id: row.get(2)?,
        name: row.get(3)?,
        sort: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn map_environment(row: &Row<'_>) -> rusqlite::Result<Result<ApiEnvironment, String>> {
    let variables_json: String = row.get(4)?;
    let environment = ApiEnvironment {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        base_url: row.get(3)?,
        variables: Vec::new(),
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    };

    Ok((|| {
        Ok(ApiEnvironment {
            variables: from_json(&variables_json)?,
            ..environment
        })
    })())
}

pub(crate) fn load_project(
    connection: &Connection,
    project_id: &str,
) -> Result<ApiProject, String> {
    connection
        .query_row(
            "SELECT id, name, description, sort, created_at, updated_at
             FROM api_projects WHERE id = ?1",
            params![project_id],
            map_project,
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {project_id} 的项目"))
}

pub(crate) fn load_group(connection: &Connection, group_id: &str) -> Result<ApiGroup, String> {
    connection
        .query_row(
            "SELECT id, project_id, parent_group_id, name, sort, created_at, updated_at
             FROM api_groups WHERE id = ?1",
            params![group_id],
            map_group,
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {group_id} 的分组"))
}

pub(crate) fn load_environment(
    connection: &Connection,
    environment_id: &str,
) -> Result<ApiEnvironment, String> {
    connection
        .query_row(
            "SELECT id, project_id, name, base_url, variables_json, created_at, updated_at
             FROM api_environments WHERE id = ?1",
            params![environment_id],
            map_environment,
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {environment_id} 的环境"))?
}

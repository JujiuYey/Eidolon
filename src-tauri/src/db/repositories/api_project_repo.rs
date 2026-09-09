//! `api_projects` 表仓库。
//!
//! 项目是接口请求工具的最高层容器，创建项目时自动生成默认分组；
//! 删除项目通过级联约束清除分组、请求与历史，返回 `DeletionSummary`
//! 给 UI 在确认框中展示。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, next_sort, require_name, sql_error, DeletionSummary,
};
use crate::models::api_client::ApiProject;

/// 创建项目时自动生成的默认分组名称
pub const DEFAULT_GROUP_NAME: &str = "默认分组";

pub struct ApiProjectRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiProjectRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<ApiProject>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, name, description, sort, created_at, updated_at
                     FROM api_projects ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let projects = statement
                .query_map([], map_project)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(projects)
        })
    }

    /// 创建项目并自动创建默认分组，两步在同一事务中完成
    pub fn create(&self, name: &str, description: &str) -> Result<ApiProject, String> {
        let name = require_name(name, "项目名称")?;
        let description = description.trim().to_string();

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_projects",
                [],
            )?;
            let project = ApiProject {
                id: format!("apj_{}", nanoid!(10)),
                name,
                description,
                sort,
                created_at: now,
                updated_at: now,
            };

            transaction
                .execute(
                    "INSERT INTO api_projects (id, name, description, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        project.id,
                        project.name,
                        project.description,
                        project.sort,
                        project.created_at,
                        project.updated_at
                    ],
                )
                .map_err(sql_error)?;

            transaction
                .execute(
                    "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                     VALUES (?1, ?2, NULL, ?3, 0, ?4, ?5)",
                    params![
                        format!("agr_{}", nanoid!(10)),
                        project.id,
                        DEFAULT_GROUP_NAME,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(project)
        })
    }

    pub fn rename(&self, project_id: &str, name: &str) -> Result<ApiProject, String> {
        let name = require_name(name, "项目名称")?;

        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let affected = connection
                .execute(
                    "UPDATE api_projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    params![name, now, project_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {project_id} 的项目"));
            }

            load_project(connection, project_id)
        })
    }

    /// 同时更新项目名称与描述。`description` 允许为空字符串
    pub fn update(
        &self,
        project_id: &str,
        name: &str,
        description: &str,
    ) -> Result<ApiProject, String> {
        let name = require_name(name, "项目名称")?;
        let description = description.trim().to_string();

        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let affected = connection
                .execute(
                    "UPDATE api_projects SET name = ?1, description = ?2, updated_at = ?3
                     WHERE id = ?4",
                    params![name, description, now, project_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {project_id} 的项目"));
            }

            load_project(connection, project_id)
        })
    }

    /// 删除项目，级联删除分组、请求与历史，返回被删除的关联数量
    pub fn delete(&self, project_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let summary = project_deletion_summary(&transaction, project_id)?;

            let affected = transaction
                .execute(
                    "DELETE FROM api_projects WHERE id = ?1",
                    params![project_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {project_id} 的项目"));
            }

            transaction.commit().map_err(sql_error)?;
            Ok(summary)
        })
    }

    /// 删除前预览关联数量，不修改数据
    pub fn preview_deletion(&self, project_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let summary = project_deletion_summary(&transaction, project_id)?;
            transaction.rollback().map_err(sql_error)?;
            Ok(summary)
        })
    }
}

fn project_deletion_summary(
    transaction: &rusqlite::Transaction<'_>,
    project_id: &str,
) -> Result<DeletionSummary, String> {
    let deleted_groups: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM api_groups WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(sql_error)?;

    let deleted_requests: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM api_requests r
             JOIN api_groups g ON g.id = r.group_id
             WHERE g.project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(sql_error)?;

    let deleted_histories: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM api_request_histories h
             JOIN api_requests r ON r.id = h.request_id
             JOIN api_groups g ON g.id = r.group_id
             WHERE g.project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(sql_error)?;

    Ok(DeletionSummary {
        deleted_groups,
        deleted_requests,
        deleted_histories,
    })
}

fn map_project(row: &Row<'_>) -> rusqlite::Result<ApiProject> {
    Ok(ApiProject {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        sort: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn load_project(connection: &Connection, project_id: &str) -> Result<ApiProject, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::api_group_repo::ApiGroupRepository;
    use crate::db::repositories::api_request_repo::ApiRequestRepository;

    fn in_memory() -> ApiClientDatabase {
        ApiClientDatabase::open_in_memory().expect("in-memory database")
    }

    #[test]
    fn create_project_also_creates_default_group() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo
            .create("订单系统", "订单相关接口")
            .expect("project should be created");

        let groups = ApiGroupRepository::new(&database)
            .list(&project.id)
            .expect("groups should load");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].project_id, project.id);
        assert_eq!(groups[0].name, DEFAULT_GROUP_NAME);
        assert_eq!(groups[0].parent_group_id, None);
    }

    #[test]
    fn create_project_rejects_blank_name() {
        let database = in_memory();
        let error = ApiProjectRepository::new(&database)
            .create("   ", "")
            .expect_err("blank name must be rejected");
        assert!(error.contains("项目名称"), "got {error}");
    }

    #[test]
    fn rename_and_delete_project_reports_cascade_counts() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo.create("订单系统", "").expect("project");
        let groups = ApiGroupRepository::new(&database)
            .list(&project.id)
            .expect("groups");
        let request_repo = ApiRequestRepository::new(&database);
        request_repo
            .create(&groups[0].id, "新增订单")
            .expect("request one");
        request_repo
            .create(&groups[0].id, "订单详情")
            .expect("request two");

        let preview = project_repo
            .preview_deletion(&project.id)
            .expect("preview should succeed");
        assert_eq!(preview.deleted_groups, 1);
        assert_eq!(preview.deleted_requests, 2);

        let renamed = project_repo
            .rename(&project.id, "订单服务")
            .expect("rename should succeed");
        assert_eq!(renamed.name, "订单服务");

        let updated = project_repo
            .update(&project.id, "订单服务", "正式环境")
            .expect("update should succeed");
        assert_eq!(updated.name, "订单服务");
        assert_eq!(updated.description, "正式环境");

        let summary = project_repo
            .delete(&project.id)
            .expect("delete should succeed");
        assert_eq!(summary.deleted_requests, 2);
        assert!(project_repo.list().expect("list").is_empty());
    }
}

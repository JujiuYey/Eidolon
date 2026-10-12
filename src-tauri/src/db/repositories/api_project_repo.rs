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
    begin, next_sort, require_name, require_project, sql_error, DeletionSummary,
};
use crate::models::api_client::{
    ApiClientOverviewStats, ApiProject, MethodCount, ProjectRequestCount, RecentExecutionStats,
};

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

    /// 读取单个项目；未命中返回 `Ok(None)`，调用方据此判断 not-found
    pub fn get(&self, project_id: &str) -> Result<Option<ApiProject>, String> {
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT id, name, description, sort, created_at, updated_at
                     FROM api_projects WHERE id = ?1",
                    params![project_id],
                    map_project,
                )
                .optional()
                .map_err(sql_error)
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

    /// 删除前评估关联数量，不修改数据。
    ///
    /// 必须确认项目存在；缺失的 `project_id` 会返回 not-found 错误，
    /// 而不是静默地返回全零计数，避免 UI 在误传 ID 时给出误导性提示
    pub fn deletion_impact(&self, project_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;
            let summary = project_deletion_summary(&transaction, project_id)?;
            transaction.rollback().map_err(sql_error)?;
            Ok(summary)
        })
    }

    /// API 管理页统计概览：各表行数、最近更新时间与窗口内新增请求时间戳。
    /// 只读聚合，`since_ms` 由命令层按「近 30 天」传入。
    pub fn overview_stats(&self, since_ms: i64) -> Result<ApiClientOverviewStats, String> {
        self.database.with_connection(|connection| {
            let count = |sql: &str| -> Result<i64, String> {
                connection
                    .query_row(sql, [], |row| row.get(0))
                    .map_err(sql_error)
            };

            let latest_updated_at: i64 = connection
                .query_row(
                    "SELECT COALESCE(MAX(updated_at), 0) FROM (
                        SELECT updated_at FROM api_projects
                        UNION ALL SELECT updated_at FROM api_groups
                        UNION ALL SELECT updated_at FROM api_requests
                        UNION ALL SELECT updated_at FROM api_environments
                    )",
                    [],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let mut statement = connection
                .prepare("SELECT created_at FROM api_requests WHERE created_at >= ?1")
                .map_err(sql_error)?;
            let recent_request_timestamps = statement
                .query_map(params![since_ms], |row| row.get(0))
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            // 方法分布（降序）
            let mut statement = connection
                .prepare(
                    "SELECT method, COUNT(*) FROM api_requests GROUP BY method ORDER BY COUNT(*) DESC, method",
                )
                .map_err(sql_error)?;
            let method_counts = statement
                .query_map([], |row| {
                    Ok(MethodCount {
                        method: row.get(0)?,
                        count: row.get(1)?,
                    })
                })
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            // 项目请求分布：LEFT JOIN 保留 0 请求的项目，降序
            let mut statement = connection
                .prepare(
                    "SELECT p.id, p.name, COUNT(r.id)
                     FROM api_projects p
                     LEFT JOIN api_groups g ON g.project_id = p.id
                     LEFT JOIN api_requests r ON r.group_id = g.id
                     GROUP BY p.id, p.name
                     ORDER BY COUNT(r.id) DESC, p.sort, p.name",
                )
                .map_err(sql_error)?;
            let project_request_counts = statement
                .query_map([], |row| {
                    Ok(ProjectRequestCount {
                        project_id: row.get(0)?,
                        project_name: row.get(1)?,
                        count: row.get(2)?,
                    })
                })
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            // 近期执行统计（每请求只保留最近 100 条历史，反映近期使用情况）
            let (total, success, avg_duration): (i64, i64, Option<f64>) = connection
                .query_row(
                    "SELECT COUNT(*),
                            COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0),
                            AVG(duration_ms)
                     FROM api_request_histories WHERE executed_at >= ?1",
                    params![since_ms],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(sql_error)?;

            let mut statement = connection
                .prepare(
                    "SELECT executed_at FROM api_request_histories WHERE executed_at >= ?1
                     ORDER BY executed_at LIMIT 50000",
                )
                .map_err(sql_error)?;
            let executed_at_timestamps = statement
                .query_map(params![since_ms], |row| row.get(0))
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(ApiClientOverviewStats {
                project_count: count("SELECT COUNT(*) FROM api_projects")?,
                group_count: count("SELECT COUNT(*) FROM api_groups")?,
                request_count: count("SELECT COUNT(*) FROM api_requests")?,
                environment_count: count("SELECT COUNT(*) FROM api_environments")?,
                latest_updated_at,
                recent_request_timestamps,
                method_counts,
                project_request_counts,
                recent_executions: RecentExecutionStats {
                    total,
                    success,
                    avg_duration_ms: avg_duration.map(|value| value.round() as i64).unwrap_or(0),
                    executed_at_timestamps,
                },
            })
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
    fn get_returns_existing_project_and_none_when_missing() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo
            .create("订单系统", "订单相关接口")
            .expect("project should be created");

        let fetched = project_repo.get(&project.id).expect("get should succeed");
        let fetched = fetched.expect("project should exist");
        assert_eq!(fetched.id, project.id);
        assert_eq!(fetched.name, "订单系统");
        assert_eq!(fetched.description, "订单相关接口");

        let missing = project_repo.get("apj_missing").expect("get should succeed");
        assert!(missing.is_none(), "missing id must surface as Ok(None)");
    }

    #[test]
    fn update_rejects_blank_name_and_missing_project() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo
            .create("订单系统", "订单相关接口")
            .expect("project should be created");

        let blank = project_repo
            .update(&project.id, "   ", "")
            .expect_err("blank name must be rejected");
        assert!(blank.contains("项目名称"), "got {blank}");

        let missing = project_repo
            .update("apj_missing", "新名称", "新描述")
            .expect_err("missing id must be rejected");
        assert!(
            missing.contains("apj_missing"),
            "missing id error must identify the id: {missing}"
        );
    }

    #[test]
    fn update_is_the_single_path_for_name_changes() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo
            .create("订单系统", "订单相关接口")
            .expect("project should be created");

        let updated = project_repo
            .update(&project.id, "订单服务", "正式环境")
            .expect("update should succeed");
        assert_eq!(updated.name, "订单服务");
        assert_eq!(updated.description, "正式环境");
        assert!(
            updated.updated_at >= project.updated_at,
            "update should refresh updated_at"
        );
    }

    #[test]
    fn deletion_impact_rejects_missing_project() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);

        let error = project_repo
            .deletion_impact("apj_missing")
            .expect_err("missing id must not silently return zeros");
        assert!(
            error.contains("apj_missing"),
            "not-found error must identify the id: {error}"
        );
    }

    #[test]
    fn update_delete_deletion_impact_reports_cascade_counts() {
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

        let impact = project_repo
            .deletion_impact(&project.id)
            .expect("deletion impact should succeed");
        assert_eq!(impact.deleted_groups, 1);
        assert_eq!(impact.deleted_requests, 2);

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

    #[test]
    fn overview_stats_counts_rows_and_buckets_recent_requests() {
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

        // 环境表直接插一行，避免引入环境仓库的构造细节
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO api_environments
                            (id, project_id, name, base_url, variables_json, created_at, updated_at)
                         VALUES ('env_1', ?1, '测试环境', '', '[]', 1, 1)",
                        params![project.id],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .expect("seed environment");

        let all = project_repo.overview_stats(0).expect("stats");
        assert_eq!(all.project_count, 1);
        assert_eq!(all.group_count, 1);
        assert_eq!(all.request_count, 2);
        assert_eq!(all.environment_count, 1);
        assert!(all.latest_updated_at > 0);
        assert_eq!(all.recent_request_timestamps.len(), 2);

        // 窗口推到未来：新增请求为空，行数不受影响
        let future = project_repo
            .overview_stats(i64::MAX / 2)
            .expect("stats with future window");
        assert!(future.recent_request_timestamps.is_empty());
        assert_eq!(future.request_count, 2);
    }

    #[test]
    fn overview_stats_reports_methods_projects_and_recent_executions() {
        let database = in_memory();
        let project_repo = ApiProjectRepository::new(&database);
        let project = project_repo.create("订单系统", "").expect("project");
        let other = project_repo.create("空项目", "").expect("empty project");
        let groups = ApiGroupRepository::new(&database)
            .list(&project.id)
            .expect("groups");
        let request_repo = ApiRequestRepository::new(&database);
        let first = request_repo
            .create(&groups[0].id, "新增订单")
            .expect("request one");
        let second = request_repo
            .create(&groups[0].id, "订单详情")
            .expect("request two");

        // 把第二个请求改成 POST，制造方法分布
        database
            .with_connection(|connection| {
                connection
                    .execute("UPDATE api_requests SET method = 'POST' WHERE id = ?1", params![second.id])
                    .map_err(|error| error.to_string())?;

                // 两条近期执行：一成一败，平均耗时 (120 + 480) / 2 = 300
                let insert_history = |id: &str, request_id: &str, status: &str, code: i64, duration: i64, at: i64| {
                    connection
                        .execute(
                            "INSERT INTO api_request_histories
                                (id, request_id, environment_name, request_snapshot_json, status, status_code,
                                 response_headers_json, response_body_preview, response_body_truncated, duration_ms,
                                 error_message, executed_at)
                             VALUES (?1, ?2, '', '{}', ?3, ?4, '[]', '', 0, ?5, NULL, ?6)",
                            params![id, request_id, status, code, duration, at],
                        )
                        .map_err(|error| error.to_string())
                };
                insert_history("h1", &first.id, "success", 200, 120, 1000)?;
                insert_history("h2", &second.id, "failed", 500, 480, 2000)?;
                Ok(())
            })
            .expect("seed method and histories");

        let stats = project_repo.overview_stats(0).expect("stats");

        assert_eq!(
            stats.method_counts,
            vec![
                MethodCount {
                    method: "GET".to_string(),
                    count: 1
                },
                MethodCount {
                    method: "POST".to_string(),
                    count: 1
                },
            ]
        );
        assert_eq!(stats.project_request_counts.len(), 2);
        assert_eq!(stats.project_request_counts[0].project_id, project.id);
        assert_eq!(stats.project_request_counts[0].count, 2);
        assert_eq!(stats.project_request_counts[1].project_id, other.id);
        assert_eq!(stats.project_request_counts[1].count, 0, "empty project must appear with 0");

        assert_eq!(stats.recent_executions.total, 2);
        assert_eq!(stats.recent_executions.success, 1);
        assert_eq!(stats.recent_executions.avg_duration_ms, 300);
        assert_eq!(stats.recent_executions.executed_at_timestamps, vec![1000, 2000]);
    }
}

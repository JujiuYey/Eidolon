use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::db::api_client::ApiClientDatabase;
use crate::models::api_client::{
    normalize_method, ApiEnvironment, ApiGroup, ApiProject, ApiRequest, BodyKind, KeyValueRow,
    RequestBody, DEFAULT_TIMEOUT_MS,
};

const DEFAULT_GROUP_NAME: &str = "默认分组";

/// 删除项目或分组时返回的关联数量，供 UI 在确认框中展示
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeletionSummary {
    pub group_count: i64,
    pub request_count: i64,
    pub history_count: i64,
}

/// 项目、分组、请求、环境的 SQLite 仓库
pub struct ApiClientRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiClientRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    // ---------- 项目 ----------

    pub fn list_projects(&self) -> Result<Vec<ApiProject>, String> {
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
    pub fn create_project(&self, name: &str, description: &str) -> Result<ApiProject, String> {
        let name = require_name(name, "项目名称")?;
        let description = description.trim().to_string();

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(&transaction, "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_projects", [])?;
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

    pub fn rename_project(&self, project_id: &str, name: &str) -> Result<ApiProject, String> {
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

    /// 删除项目，级联删除分组、请求与历史，返回被删除的关联数量
    pub fn delete_project(&self, project_id: &str) -> Result<DeletionSummary, String> {
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

            transaction
                .execute(
                    "INSERT INTO api_project_deletions
                        (project_id, group_count, request_count, history_count, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (project_id) DO UPDATE SET
                        group_count = excluded.group_count,
                        request_count = excluded.request_count,
                        history_count = excluded.history_count,
                        deleted_at = excluded.deleted_at",
                    params![
                        project_id,
                        summary.group_count,
                        summary.request_count,
                        summary.history_count,
                        Utc::now().timestamp_millis()
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(summary)
        })
    }

    /// 删除前预览关联数量，不修改数据
    pub fn preview_project_deletion(&self, project_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let summary = project_deletion_summary(&transaction, project_id)?;
            transaction.rollback().map_err(sql_error)?;
            Ok(summary)
        })
    }

    // ---------- 分组 ----------

    pub fn list_groups(&self, project_id: &str) -> Result<Vec<ApiGroup>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, project_id, parent_group_id, name, sort, created_at, updated_at
                     FROM api_groups WHERE project_id = ?1 ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let groups = statement
                .query_map(params![project_id], map_group)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(groups)
        })
    }

    pub fn create_group(&self, project_id: &str, name: &str) -> Result<ApiGroup, String> {
        let name = require_name(name, "分组名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_groups WHERE project_id = ?1",
                params![project_id],
            )?;

            let group = ApiGroup {
                id: format!("agr_{}", nanoid!(10)),
                project_id: project_id.to_string(),
                parent_group_id: None,
                name,
                sort,
                created_at: now,
                updated_at: now,
            };

            transaction
                .execute(
                    "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
                    params![
                        group.id,
                        group.project_id,
                        group.name,
                        group.sort,
                        group.created_at,
                        group.updated_at
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(group)
        })
    }

    pub fn rename_group(&self, group_id: &str, name: &str) -> Result<ApiGroup, String> {
        let name = require_name(name, "分组名称")?;

        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let affected = connection
                .execute(
                    "UPDATE api_groups SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    params![name, now, group_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {group_id} 的分组"));
            }

            load_group(connection, group_id)
        })
    }

    pub fn delete_group(&self, group_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let request_count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM api_requests WHERE group_id = ?1",
                    params![group_id],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let history_count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM api_request_histories h
                     JOIN api_requests r ON r.id = h.request_id
                     WHERE r.group_id = ?1",
                    params![group_id],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let affected = transaction
                .execute("DELETE FROM api_groups WHERE id = ?1", params![group_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {group_id} 的分组"));
            }

            transaction.commit().map_err(sql_error)?;

            Ok(DeletionSummary {
                group_count: 1,
                request_count,
                history_count,
            })
        })
    }

    /// 按给定顺序重排分组，顺序稳定且只影响该项目
    pub fn reorder_groups(
        &self,
        project_id: &str,
        group_ids: &[String],
    ) -> Result<Vec<ApiGroup>, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let now = Utc::now().timestamp_millis();

            for (index, group_id) in group_ids.iter().enumerate() {
                let affected = transaction
                    .execute(
                        "UPDATE api_groups SET sort = ?1, updated_at = ?2
                         WHERE id = ?3 AND project_id = ?4",
                        params![index as i64, now, group_id, project_id],
                    )
                    .map_err(sql_error)?;

                if affected == 0 {
                    return Err(format!("分组 {group_id} 不属于项目 {project_id}"));
                }
            }

            transaction.commit().map_err(sql_error)?;
            self.list_groups_internal(project_id)
        })
    }

    fn list_groups_internal(&self, project_id: &str) -> Result<Vec<ApiGroup>, String> {
        self.list_groups(project_id)
    }

    // ---------- 请求 ----------

    pub fn list_requests(&self, project_id: &str) -> Result<Vec<ApiRequest>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT r.id, r.group_id, g.project_id, r.name, r.method, r.url, r.query_json,
                            r.headers_json, r.body_kind, r.body_text, r.body_form_json, r.timeout_ms,
                            r.ai_prompt, r.ai_reference, r.sort, r.created_at, r.updated_at
                     FROM api_requests r
                     JOIN api_groups g ON g.id = r.group_id
                     WHERE g.project_id = ?1
                     ORDER BY g.sort, r.sort, r.created_at, r.id",
                )
                .map_err(sql_error)?;

            let requests = statement
                .query_map(params![project_id], map_request)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            requests.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_request(&self, request_id: &str) -> Result<Option<ApiRequest>, String> {
        self.database
            .with_connection(|connection| load_request_optional(connection, request_id))
    }

    pub fn create_request(&self, group_id: &str, name: &str) -> Result<ApiRequest, String> {
        let name = require_name(name, "请求名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let project_id = group_project_id(&transaction, group_id)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_requests WHERE group_id = ?1",
                params![group_id],
            )?;

            let request = ApiRequest {
                id: format!("areq_{}", nanoid!(10)),
                group_id: group_id.to_string(),
                project_id,
                name,
                method: "GET".to_string(),
                url: String::new(),
                query: Vec::new(),
                headers: Vec::new(),
                body: RequestBody::default(),
                timeout_ms: DEFAULT_TIMEOUT_MS,
                ai_prompt: String::new(),
                ai_reference: String::new(),
                sort,
                created_at: now,
                updated_at: now,
            };

            insert_request(&transaction, &request)?;
            transaction.commit().map_err(sql_error)?;
            Ok(request)
        })
    }

    /// 保存请求定义，包含 AI 生成要求与参考内容
    pub fn update_request(&self, request: &ApiRequest) -> Result<ApiRequest, String> {
        let name = require_name(&request.name, "请求名称")?;
        let method = normalize_method(&request.method)?;
        let timeout_ms = require_timeout(request.timeout_ms)?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let project_id = group_project_id(&transaction, &request.group_id)?;
            let now = Utc::now().timestamp_millis();

            let affected = transaction
                .execute(
                    "UPDATE api_requests SET
                        group_id = ?1, name = ?2, method = ?3, url = ?4, query_json = ?5,
                        headers_json = ?6, body_kind = ?7, body_text = ?8, body_form_json = ?9,
                        timeout_ms = ?10, ai_prompt = ?11, ai_reference = ?12, updated_at = ?13
                     WHERE id = ?14",
                    params![
                        request.group_id,
                        name,
                        method,
                        request.url.trim(),
                        to_json(&request.query)?,
                        to_json(&request.headers)?,
                        request.body.kind.as_str(),
                        request.body.text,
                        to_json(&request.body.form)?,
                        timeout_ms as i64,
                        request.ai_prompt,
                        request.ai_reference,
                        now,
                        request.id
                    ],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {} 的请求", request.id));
            }

            transaction.commit().map_err(sql_error)?;

            let mut saved = load_request(connection, &request.id)?;
            saved.project_id = project_id;
            Ok(saved)
        })
    }

    pub fn duplicate_request(&self, request_id: &str) -> Result<ApiRequest, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let source = load_request_tx(&transaction, request_id)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_requests WHERE group_id = ?1",
                params![source.group_id],
            )?;

            let copy = ApiRequest {
                id: format!("areq_{}", nanoid!(10)),
                name: format!("{} 副本", source.name),
                sort,
                created_at: now,
                updated_at: now,
                ..source
            };

            insert_request(&transaction, &copy)?;
            transaction.commit().map_err(sql_error)?;
            Ok(copy)
        })
    }

    /// 移动请求到同项目的其他分组；跨项目移动被拒绝
    pub fn move_request(
        &self,
        request_id: &str,
        target_group_id: &str,
    ) -> Result<ApiRequest, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let request = load_request_tx(&transaction, request_id)?;
            let source_project = group_project_id(&transaction, &request.group_id)?;
            let target_project = group_project_id(&transaction, target_group_id)?;

            if source_project != target_project {
                return Err("不能把请求移动到其他项目的分组".to_string());
            }

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_requests WHERE group_id = ?1",
                params![target_group_id],
            )?;

            transaction
                .execute(
                    "UPDATE api_requests SET group_id = ?1, sort = ?2, updated_at = ?3 WHERE id = ?4",
                    params![target_group_id, sort, now, request_id],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            load_request(connection, request_id)
        })
    }

    pub fn delete_request(&self, request_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let history_count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM api_request_histories WHERE request_id = ?1",
                    params![request_id],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let affected = transaction
                .execute(
                    "DELETE FROM api_requests WHERE id = ?1",
                    params![request_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {request_id} 的请求"));
            }

            transaction.commit().map_err(sql_error)?;

            Ok(DeletionSummary {
                group_count: 0,
                request_count: 1,
                history_count,
            })
        })
    }

    pub fn reorder_requests(
        &self,
        group_id: &str,
        request_ids: &[String],
    ) -> Result<Vec<ApiRequest>, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let now = Utc::now().timestamp_millis();

            for (index, request_id) in request_ids.iter().enumerate() {
                let affected = transaction
                    .execute(
                        "UPDATE api_requests SET sort = ?1, updated_at = ?2
                         WHERE id = ?3 AND group_id = ?4",
                        params![index as i64, now, request_id, group_id],
                    )
                    .map_err(sql_error)?;

                if affected == 0 {
                    return Err(format!("请求 {request_id} 不属于分组 {group_id}"));
                }
            }

            let project_id = group_project_id(&transaction, group_id)?;
            transaction.commit().map_err(sql_error)?;

            let requests = self.list_requests(&project_id)?;
            Ok(requests
                .into_iter()
                .filter(|request| request.group_id == group_id)
                .collect())
        })
    }

    // ---------- 环境 ----------

    pub fn list_environments(&self, project_id: &str) -> Result<Vec<ApiEnvironment>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, project_id, name, base_url, variables_json, created_at, updated_at
                     FROM api_environments WHERE project_id = ?1 ORDER BY name, id",
                )
                .map_err(sql_error)?;

            let environments = statement
                .query_map(params![project_id], map_environment)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            environments.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn upsert_environment(
        &self,
        environment: &ApiEnvironment,
    ) -> Result<ApiEnvironment, String> {
        let name = require_name(&environment.name, "环境名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, &environment.project_id)?;

            let now = Utc::now().timestamp_millis();
            let id = if environment.id.trim().is_empty() {
                format!("aenv_{}", nanoid!(10))
            } else {
                environment.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO api_environments
                        (id, project_id, name, base_url, variables_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        base_url = excluded.base_url,
                        variables_json = excluded.variables_json,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        environment.project_id,
                        name,
                        environment.base_url.trim(),
                        to_json(&environment.variables)?,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            load_environment(connection, &id)
        })
    }

    pub fn delete_environment(&self, environment_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM api_environments WHERE id = ?1",
                    params![environment_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {environment_id} 的环境"));
            }

            Ok(environment_id.to_string())
        })
    }

    /// 校验环境与请求属于同一项目，返回环境
    pub fn resolve_environment_for_request(
        &self,
        request_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Option<ApiEnvironment>, String> {
        let Some(environment_id) = environment_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return Ok(None);
        };

        self.database.with_connection(|connection| {
            let request_project: String = connection
                .query_row(
                    "SELECT g.project_id FROM api_requests r
                     JOIN api_groups g ON g.id = r.group_id
                     WHERE r.id = ?1",
                    params![request_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_error)?
                .ok_or_else(|| format!("未找到 id 为 {request_id} 的请求"))?;

            let environment = load_environment(connection, environment_id)?;

            if environment.project_id != request_project {
                return Err("所选环境与请求不属于同一项目".to_string());
            }

            Ok(Some(environment))
        })
    }
}

// ---------- 内部帮助函数 ----------

fn begin(connection: &mut Connection) -> Result<Transaction<'_>, String> {
    connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("开始事务失败: {error}"))
}

fn sql_error(error: rusqlite::Error) -> String {
    format!("数据库操作失败: {error}")
}

fn require_name(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    Ok(trimmed.to_string())
}

fn require_timeout(timeout_ms: u64) -> Result<u64, String> {
    if timeout_ms == 0 {
        return Err("超时时间必须大于 0".to_string());
    }
    Ok(timeout_ms)
}

fn next_sort<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    sql: &str,
    params: P,
) -> Result<i64, String> {
    transaction
        .query_row(sql, params, |row| row.get(0))
        .map_err(sql_error)
}

fn require_project(transaction: &Transaction<'_>, project_id: &str) -> Result<(), String> {
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

fn group_project_id(transaction: &Transaction<'_>, group_id: &str) -> Result<String, String> {
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

fn project_deletion_summary(
    transaction: &Transaction<'_>,
    project_id: &str,
) -> Result<DeletionSummary, String> {
    let group_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM api_groups WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(sql_error)?;

    let request_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM api_requests r
             JOIN api_groups g ON g.id = r.group_id
             WHERE g.project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(sql_error)?;

    let history_count: i64 = transaction
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
        group_count,
        request_count,
        history_count,
    })
}

fn insert_request(transaction: &Transaction<'_>, request: &ApiRequest) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO api_requests
                (id, group_id, name, method, url, query_json, headers_json, body_kind, body_text,
                 body_form_json, timeout_ms, ai_prompt, ai_reference, sort, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                request.id,
                request.group_id,
                request.name,
                request.method,
                request.url,
                to_json(&request.query)?,
                to_json(&request.headers)?,
                request.body.kind.as_str(),
                request.body.text,
                to_json(&request.body.form)?,
                request.timeout_ms as i64,
                request.ai_prompt,
                request.ai_reference,
                request.sort,
                request.created_at,
                request.updated_at
            ],
        )
        .map_err(sql_error)?;

    Ok(())
}

fn to_json(rows: &[KeyValueRow]) -> Result<String, String> {
    serde_json::to_string(rows).map_err(|error| format!("序列化键值行失败: {error}"))
}

fn from_json(value: &str) -> Result<Vec<KeyValueRow>, String> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value).map_err(|error| format!("解析键值行失败: {error}"))
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

fn map_group(row: &Row<'_>) -> rusqlite::Result<ApiGroup> {
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

fn map_request(row: &Row<'_>) -> rusqlite::Result<Result<ApiRequest, String>> {
    let query_json: String = row.get(6)?;
    let headers_json: String = row.get(7)?;
    let body_kind: String = row.get(8)?;
    let body_form_json: String = row.get(10)?;
    let timeout_ms: i64 = row.get(11)?;

    let parsed = (|| {
        Ok(ApiRequest {
            id: row.get(0)?,
            group_id: row.get(1)?,
            project_id: row.get(2)?,
            name: row.get(3)?,
            method: row.get(4)?,
            url: row.get(5)?,
            query: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::default(),
            timeout_ms: timeout_ms.max(0) as u64,
            ai_prompt: row.get(12)?,
            ai_reference: row.get(13)?,
            sort: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    })();

    let mut request: ApiRequest = match parsed {
        Ok(request) => request,
        Err(error) => return Err(error),
    };

    Ok((|| {
        request.query = from_json(&query_json)?;
        request.headers = from_json(&headers_json)?;
        request.body = RequestBody {
            kind: BodyKind::parse(&body_kind)?,
            text: String::new(),
            form: from_json(&body_form_json)?,
        };
        Ok(request)
    })())
}

fn request_columns() -> &'static str {
    "r.id, r.group_id, g.project_id, r.name, r.method, r.url, r.query_json, r.headers_json,
     r.body_kind, r.body_text, r.body_form_json, r.timeout_ms, r.ai_prompt, r.ai_reference,
     r.sort, r.created_at, r.updated_at"
}

fn read_request(row: &Row<'_>) -> rusqlite::Result<Result<ApiRequest, String>> {
    let body_text: String = row.get(9)?;
    let mapped = map_request(row)?;
    Ok(mapped.map(|mut request| {
        request.body.text = body_text;
        request
    }))
}

fn load_request_optional(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<ApiRequest>, String> {
    let sql = format!(
        "SELECT {} FROM api_requests r JOIN api_groups g ON g.id = r.group_id WHERE r.id = ?1",
        request_columns()
    );

    let result = connection
        .query_row(&sql, params![request_id], read_request)
        .optional()
        .map_err(sql_error)?;

    match result {
        Some(inner) => inner.map(Some),
        None => Ok(None),
    }
}

fn load_request(connection: &Connection, request_id: &str) -> Result<ApiRequest, String> {
    load_request_optional(connection, request_id)?
        .ok_or_else(|| format!("未找到 id 为 {request_id} 的请求"))
}

fn load_request_tx(transaction: &Transaction<'_>, request_id: &str) -> Result<ApiRequest, String> {
    let sql = format!(
        "SELECT {} FROM api_requests r JOIN api_groups g ON g.id = r.group_id WHERE r.id = ?1",
        request_columns()
    );

    transaction
        .query_row(&sql, params![request_id], read_request)
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {request_id} 的请求"))?
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

fn load_group(connection: &Connection, group_id: &str) -> Result<ApiGroup, String> {
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

fn map_environment(row: &Row<'_>) -> rusqlite::Result<Result<ApiEnvironment, String>> {
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

fn load_environment(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn repository() -> (ApiClientDatabase, ()) {
        (
            ApiClientDatabase::open_in_memory().expect("in-memory database"),
            (),
        )
    }

    fn seeded_project(repo: &ApiClientRepository<'_>) -> (ApiProject, ApiGroup) {
        let project = repo
            .create_project("订单系统", "订单相关接口")
            .expect("project should be created");
        let groups = repo.list_groups(&project.id).expect("groups should load");
        (project, groups[0].clone())
    }

    #[test]
    fn create_project_also_creates_default_group() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);

        let (project, group) = seeded_project(&repo);

        assert_eq!(group.project_id, project.id);
        assert_eq!(group.name, DEFAULT_GROUP_NAME);
        assert_eq!(group.parent_group_id, None);
        assert_eq!(repo.list_groups(&project.id).expect("groups").len(), 1);
    }

    #[test]
    fn create_project_rejects_blank_name() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);

        let error = repo
            .create_project("   ", "")
            .expect_err("blank name must be rejected");
        assert!(error.contains("项目名称"), "got {error}");
    }

    #[test]
    fn request_round_trips_all_fields_including_ai_inputs() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (project, group) = seeded_project(&repo);

        let created = repo
            .create_request(&group.id, "新增订单")
            .expect("request should be created");

        let saved = repo
            .update_request(&ApiRequest {
                method: "post".to_string(),
                url: "{{base_url}}/orders".to_string(),
                query: vec![
                    KeyValueRow::new("tag", "a"),
                    KeyValueRow::new("tag", "b"),
                    KeyValueRow::disabled("debug", "1"),
                ],
                headers: vec![KeyValueRow::new("Authorization", "Bearer {{token}}")],
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{\"items\":3}".to_string(),
                    form: Vec::new(),
                },
                timeout_ms: 15_000,
                ai_prompt: "生成一个包含三件商品的订单".to_string(),
                ai_reference: "字段说明：items 为数组".to_string(),
                ..created.clone()
            })
            .expect("request should be saved");

        assert_eq!(saved.method, "POST");
        assert_eq!(saved.project_id, project.id);
        assert_eq!(saved.query.len(), 3, "duplicate keys must be preserved");
        assert_eq!(saved.query[0].key, "tag");
        assert_eq!(saved.query[1].value, "b");
        assert!(!saved.query[2].enabled, "disabled rows keep their state");
        assert_eq!(saved.body.kind, BodyKind::Json);
        assert_eq!(saved.body.text, "{\"items\":3}");
        assert_eq!(saved.timeout_ms, 15_000);
        assert_eq!(saved.ai_prompt, "生成一个包含三件商品的订单");
        assert_eq!(saved.ai_reference, "字段说明：items 为数组");

        let reloaded = repo
            .get_request(&created.id)
            .expect("lookup should succeed")
            .expect("request should exist");
        assert_eq!(reloaded, saved);
    }

    #[test]
    fn update_request_rejects_unsupported_method_and_blank_name() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (_project, group) = seeded_project(&repo);
        let created = repo.create_request(&group.id, "新增订单").expect("created");

        let method_error = repo
            .update_request(&ApiRequest {
                method: "TRACE".to_string(),
                ..created.clone()
            })
            .expect_err("unsupported method must be rejected");
        assert!(
            method_error.contains("不支持的 HTTP 方法"),
            "got {method_error}"
        );

        let name_error = repo
            .update_request(&ApiRequest {
                name: " ".to_string(),
                ..created
            })
            .expect_err("blank name must be rejected");
        assert!(name_error.contains("请求名称"), "got {name_error}");
    }

    #[test]
    fn move_request_across_projects_is_rejected() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (_first_project, first_group) = seeded_project(&repo);
        let second_project = repo
            .create_project("库存系统", "")
            .expect("second project should be created");
        let second_group = repo
            .list_groups(&second_project.id)
            .expect("groups should load")[0]
            .clone();

        let request = repo
            .create_request(&first_group.id, "新增订单")
            .expect("request should be created");

        let error = repo
            .move_request(&request.id, &second_group.id)
            .expect_err("cross-project move must be rejected");
        assert!(error.contains("其他项目"), "got {error}");

        let same_project_group = repo
            .create_group(&_first_project.id, "订单查询")
            .expect("group should be created");
        let moved = repo
            .move_request(&request.id, &same_project_group.id)
            .expect("same-project move should succeed");
        assert_eq!(moved.group_id, same_project_group.id);
    }

    #[test]
    fn reorder_groups_is_stable_and_scoped_to_project() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (project, default_group) = seeded_project(&repo);
        let second = repo.create_group(&project.id, "订单查询").expect("group b");
        let third = repo.create_group(&project.id, "订单统计").expect("group c");

        let reordered = repo
            .reorder_groups(
                &project.id,
                &[
                    third.id.clone(),
                    default_group.id.clone(),
                    second.id.clone(),
                ],
            )
            .expect("reorder should succeed");

        let ids = reordered
            .iter()
            .map(|group| group.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                third.id.as_str(),
                default_group.id.as_str(),
                second.id.as_str()
            ]
        );

        let other_project = repo.create_project("库存系统", "").expect("other project");
        let error = repo
            .reorder_groups(&other_project.id, &[second.id.clone()])
            .expect_err("group from another project must be rejected");
        assert!(error.contains("不属于项目"), "got {error}");
    }

    #[test]
    fn duplicate_request_copies_configuration_into_new_id() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (_project, group) = seeded_project(&repo);
        let created = repo.create_request(&group.id, "新增订单").expect("created");
        let saved = repo
            .update_request(&ApiRequest {
                url: "/orders".to_string(),
                headers: vec![KeyValueRow::new("X-Tenant", "1")],
                ..created
            })
            .expect("saved");

        let copy = repo
            .duplicate_request(&saved.id)
            .expect("copy should be created");

        assert_ne!(copy.id, saved.id);
        assert_eq!(copy.name, "新增订单 副本");
        assert_eq!(copy.url, "/orders");
        assert_eq!(copy.headers, saved.headers);
    }

    #[test]
    fn delete_project_reports_cascade_counts() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (project, group) = seeded_project(&repo);
        repo.create_request(&group.id, "新增订单")
            .expect("request one");
        repo.create_request(&group.id, "订单详情")
            .expect("request two");

        let preview = repo
            .preview_project_deletion(&project.id)
            .expect("preview should succeed");
        assert_eq!(preview.group_count, 1);
        assert_eq!(preview.request_count, 2);

        let summary = repo
            .delete_project(&project.id)
            .expect("delete should succeed");
        assert_eq!(summary.request_count, 2);
        assert!(repo.list_projects().expect("list").is_empty());
        assert!(repo
            .list_requests(&project.id)
            .expect("requests")
            .is_empty());
    }

    #[test]
    fn environment_upsert_and_ownership_validation() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (project, group) = seeded_project(&repo);
        let request = repo.create_request(&group.id, "新增订单").expect("request");

        let environment = repo
            .upsert_environment(&ApiEnvironment {
                project_id: project.id.clone(),
                name: "测试环境".to_string(),
                base_url: "https://test.example.com".to_string(),
                variables: vec![KeyValueRow::new("token", "abc")],
                ..Default::default()
            })
            .expect("environment should be created");

        assert_eq!(environment.variables.len(), 1);

        let renamed = repo
            .upsert_environment(&ApiEnvironment {
                name: "预发环境".to_string(),
                ..environment.clone()
            })
            .expect("environment should update in place");
        assert_eq!(renamed.id, environment.id);
        assert_eq!(renamed.name, "预发环境");
        assert_eq!(repo.list_environments(&project.id).expect("list").len(), 1);

        let resolved = repo
            .resolve_environment_for_request(&request.id, Some(&environment.id))
            .expect("same-project environment should resolve")
            .expect("environment should be present");
        assert_eq!(resolved.id, environment.id);

        let other_project = repo.create_project("库存系统", "").expect("other project");
        let foreign_environment = repo
            .upsert_environment(&ApiEnvironment {
                project_id: other_project.id,
                name: "测试环境".to_string(),
                ..Default::default()
            })
            .expect("foreign environment should be created");

        let error = repo
            .resolve_environment_for_request(&request.id, Some(&foreign_environment.id))
            .expect_err("foreign environment must be rejected");
        assert!(error.contains("同一项目"), "got {error}");

        assert!(repo
            .resolve_environment_for_request(&request.id, None)
            .expect("no environment is allowed")
            .is_none());
    }

    #[test]
    fn delete_group_reports_request_and_history_counts() {
        let (database, _) = repository();
        let repo = ApiClientRepository::new(&database);
        let (project, group) = seeded_project(&repo);
        repo.create_request(&group.id, "新增订单").expect("request");

        let summary = repo.delete_group(&group.id).expect("delete should succeed");
        assert_eq!(summary.group_count, 1);
        assert_eq!(summary.request_count, 1);
        assert!(repo.list_groups(&project.id).expect("groups").is_empty());
    }
}

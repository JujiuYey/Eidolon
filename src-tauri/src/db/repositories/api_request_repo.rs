//! `api_requests` 表仓库。
//!
//! 请求通过分组归属项目；保存包含 AI 生成要求与参考内容。
//! 跨项目移动请求与环境解析也由本仓库处理，因为请求是关联主体。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, from_json, group_project_id, load_environment, next_sort, require_name, require_timeout,
    sql_error, to_json, DeletionSummary,
};
use crate::models::api_client::{
    normalize_method, ApiEnvironment, ApiRequest, BodyKind, RequestBody, DEFAULT_TIMEOUT_MS,
};

pub struct ApiRequestRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiRequestRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self, project_id: &str) -> Result<Vec<ApiRequest>, String> {
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
                .query_map(params![project_id], read_request)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            requests.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get(&self, request_id: &str) -> Result<Option<ApiRequest>, String> {
        self.database
            .with_connection(|connection| load_request_optional(connection, request_id))
    }

    pub fn create(&self, group_id: &str, name: &str) -> Result<ApiRequest, String> {
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
    pub fn update(&self, request: &ApiRequest) -> Result<ApiRequest, String> {
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

    pub fn duplicate(&self, request_id: &str) -> Result<ApiRequest, String> {
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
    pub fn move_to_group(
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

    pub fn delete(&self, request_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let deleted_histories: i64 = transaction
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
                deleted_groups: 0,
                deleted_requests: 1,
                deleted_histories,
            })
        })
    }

    pub fn reorder(
        &self,
        group_id: &str,
        request_ids: &[String],
    ) -> Result<Vec<ApiRequest>, String> {
        let project_id = self.database.with_connection(|connection| {
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
            Ok(project_id)
        })?;

        let requests = self.list(&project_id)?;
        Ok(requests
            .into_iter()
            .filter(|request| request.group_id == group_id)
            .collect())
    }

    /// 校验环境与请求属于同一项目，返回环境
    pub fn resolve_environment(
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

fn request_columns() -> &'static str {
    "r.id, r.group_id, g.project_id, r.name, r.method, r.url, r.query_json, r.headers_json,
     r.body_kind, r.body_text, r.body_form_json, r.timeout_ms, r.ai_prompt, r.ai_reference,
     r.sort, r.created_at, r.updated_at"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::api_environment_repo::ApiEnvironmentRepository;
    use crate::db::repositories::api_group_repo::ApiGroupRepository;
    use crate::db::repositories::api_project_repo::ApiProjectRepository;
    use crate::models::api_client::{BodyKind, KeyValueRow, RequestBody};

    fn seeded(database: &ApiClientDatabase) -> (String, String) {
        let project = ApiProjectRepository::new(database)
            .create("订单系统", "订单相关接口")
            .expect("project");
        let group = ApiGroupRepository::new(database)
            .list(&project.id)
            .expect("groups")[0]
            .clone();
        (project.id, group.id)
    }

    #[test]
    fn request_round_trips_all_fields_including_ai_inputs() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let (project_id, group_id) = seeded(&database);
        let repo = ApiRequestRepository::new(&database);

        let created = repo
            .create(&group_id, "新增订单")
            .expect("request should be created");

        let saved = repo
            .update(&ApiRequest {
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
        assert_eq!(saved.project_id, project_id);
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
            .get(&created.id)
            .expect("lookup should succeed")
            .expect("request should exist");
        assert_eq!(reloaded, saved);
    }

    #[test]
    fn update_request_rejects_unsupported_method_and_blank_name() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let (_, group_id) = seeded(&database);
        let repo = ApiRequestRepository::new(&database);
        let created = repo.create(&group_id, "新增订单").expect("created");

        let method_error = repo
            .update(&ApiRequest {
                method: "TRACE".to_string(),
                ..created.clone()
            })
            .expect_err("unsupported method must be rejected");
        assert!(
            method_error.contains("不支持的 HTTP 方法"),
            "got {method_error}"
        );

        let name_error = repo
            .update(&ApiRequest {
                name: " ".to_string(),
                ..created
            })
            .expect_err("blank name must be rejected");
        assert!(name_error.contains("请求名称"), "got {name_error}");
    }

    #[test]
    fn move_request_across_projects_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let project_repo = ApiProjectRepository::new(&database);
        let group_repo = ApiGroupRepository::new(&database);
        let (first_project_id, first_group_id) = seeded(&database);
        let second_project = project_repo
            .create("库存系统", "")
            .expect("second project should be created");
        let second_group = group_repo.list(&second_project.id).expect("groups")[0].clone();

        let request = ApiRequestRepository::new(&database)
            .create(&first_group_id, "新增订单")
            .expect("request should be created");

        let error = ApiRequestRepository::new(&database)
            .move_to_group(&request.id, &second_group.id)
            .expect_err("cross-project move must be rejected");
        assert!(error.contains("其他项目"), "got {error}");

        let same_project_group = group_repo
            .create(&first_project_id, "订单查询")
            .expect("group should be created");
        let moved = ApiRequestRepository::new(&database)
            .move_to_group(&request.id, &same_project_group.id)
            .expect("same-project move should succeed");
        assert_eq!(moved.group_id, same_project_group.id);
    }

    #[test]
    fn duplicate_request_copies_configuration_into_new_id() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let (_, group_id) = seeded(&database);
        let repo = ApiRequestRepository::new(&database);
        let created = repo.create(&group_id, "新增订单").expect("created");
        let saved = repo
            .update(&ApiRequest {
                url: "/orders".to_string(),
                headers: vec![KeyValueRow::new("X-Tenant", "1")],
                ..created
            })
            .expect("saved");

        let copy = repo.duplicate(&saved.id).expect("copy should be created");

        assert_ne!(copy.id, saved.id);
        assert_eq!(copy.name, "新增订单 副本");
        assert_eq!(copy.url, "/orders");
        assert_eq!(copy.headers, saved.headers);
    }

    #[test]
    fn delete_request_reports_history_count() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let (_, group_id) = seeded(&database);
        let repo = ApiRequestRepository::new(&database);
        let request = repo.create(&group_id, "新增订单").expect("request");
        let history_repo =
            crate::db::repositories::api_history::ApiHistoryRepository::new(&database);
        history_repo
            .append(&crate::models::api_client::ApiRequestHistory {
                request_id: request.id.clone(),
                request_snapshot: crate::models::api_client::RequestSnapshot::default(),
                status: crate::models::api_client::ExecutionStatus::Success,
                ..Default::default()
            })
            .expect("history should be written");

        let summary = repo.delete(&request.id).expect("delete should succeed");
        assert_eq!(summary.deleted_requests, 1);
        assert_eq!(summary.deleted_histories, 1);
        assert!(repo.get(&request.id).expect("get").is_none());
    }

    #[test]
    fn resolve_environment_rejects_cross_project_use() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let project_repo = ApiProjectRepository::new(&database);
        let env_repo = ApiEnvironmentRepository::new(&database);
        let (project_id, group_id) = seeded(&database);

        let request = ApiRequestRepository::new(&database)
            .create(&group_id, "新增订单")
            .expect("request");

        let env = env_repo
            .upsert(&crate::models::api_client::ApiEnvironment {
                project_id: project_id.clone(),
                name: "测试环境".to_string(),
                base_url: "https://test.example.com".to_string(),
                ..Default::default()
            })
            .expect("environment should be created");

        let resolved = ApiRequestRepository::new(&database)
            .resolve_environment(&request.id, Some(&env.id))
            .expect("same-project environment should resolve")
            .expect("environment should be present");
        assert_eq!(resolved.id, env.id);

        let foreign_env = env_repo
            .upsert(&crate::models::api_client::ApiEnvironment {
                project_id: project_repo
                    .create("库存系统", "")
                    .expect("other project")
                    .id,
                name: "测试环境".to_string(),
                ..Default::default()
            })
            .expect("foreign environment should be created");

        let error = ApiRequestRepository::new(&database)
            .resolve_environment(&request.id, Some(&foreign_env.id))
            .expect_err("foreign environment must be rejected");
        assert!(error.contains("同一项目"), "got {error}");

        assert!(ApiRequestRepository::new(&database)
            .resolve_environment(&request.id, None)
            .expect("no environment is allowed")
            .is_none());
    }
}

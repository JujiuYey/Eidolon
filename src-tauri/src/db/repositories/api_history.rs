use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::models::api_client::{
    ApiRequestHistory, ExecutionStatus, KeyValueRow, RequestSnapshot, MAX_HISTORY_BODY_BYTES,
    MAX_HISTORY_PER_REQUEST,
};

/// 认证类请求头名称片段，大小写不敏感匹配后脱敏
const SENSITIVE_HEADER_HINTS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "proxy-authorization",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "x-access-token",
    "token",
    "secret",
    "password",
];

/// 脱敏后的占位值
const REDACTED: &str = "***";

/// 执行历史仓库
pub struct ApiHistoryRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiHistoryRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self, request_id: &str) -> Result<Vec<ApiRequestHistory>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, request_id, environment_name, request_snapshot_json, status,
                            status_code, response_headers_json, response_body_preview,
                            response_body_truncated, duration_ms, error_message, executed_at
                     FROM api_request_histories
                     WHERE request_id = ?1
                     ORDER BY executed_at DESC, id DESC",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map(params![request_id], map_history)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    /// 写入一条历史：请求快照与响应体在写入前脱敏、截断
    pub fn append(&self, history: &ApiRequestHistory) -> Result<ApiRequestHistory, String> {
        let mut record = history.clone();

        if record.id.trim().is_empty() {
            record.id = format!("ahis_{}", nanoid!(10));
        }

        if record.executed_at == 0 {
            record.executed_at = Utc::now().timestamp_millis();
        }

        record.request_snapshot = redact_snapshot(&record.request_snapshot);
        record.response_headers = redact_rows(&record.response_headers);

        let (preview, truncated) = truncate_body(&record.response_body_preview);
        record.response_body_preview = preview;
        record.response_body_truncated = record.response_body_truncated || truncated;

        self.database.with_connection(|connection| {
            let transaction = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|error| format!("开始事务失败: {error}"))?;

            transaction
                .execute(
                    "INSERT INTO api_request_histories
                        (id, request_id, environment_name, request_snapshot_json, status, status_code,
                         response_headers_json, response_body_preview, response_body_truncated,
                         duration_ms, error_message, executed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        record.id,
                        record.request_id,
                        record.environment_name,
                        serde_json::to_string(&record.request_snapshot)
                            .map_err(|error| format!("序列化请求快照失败: {error}"))?,
                        record.status.as_str(),
                        record.status_code.map(|code| code as i64),
                        serde_json::to_string(&record.response_headers)
                            .map_err(|error| format!("序列化响应头失败: {error}"))?,
                        record.response_body_preview,
                        record.response_body_truncated as i64,
                        record.duration_ms as i64,
                        record.error_message,
                        record.executed_at
                    ],
                )
                .map_err(sql_error)?;

            // 只保留最近 MAX_HISTORY_PER_REQUEST 条
            transaction
                .execute(
                    "DELETE FROM api_request_histories
                     WHERE request_id = ?1 AND id NOT IN (
                        SELECT id FROM api_request_histories
                        WHERE request_id = ?1
                        ORDER BY executed_at DESC, id DESC
                        LIMIT ?2
                     )",
                    params![record.request_id, MAX_HISTORY_PER_REQUEST as i64],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(record)
        })
    }

    pub fn clear(&self, request_id: &str) -> Result<usize, String> {
        self.database.with_connection(|connection| {
            connection
                .execute(
                    "DELETE FROM api_request_histories WHERE request_id = ?1",
                    params![request_id],
                )
                .map_err(sql_error)
        })
    }

    pub fn get(&self, history_id: &str) -> Result<Option<ApiRequestHistory>, String> {
        self.database.with_connection(|connection| {
            let result = connection
                .query_row(
                    "SELECT id, request_id, environment_name, request_snapshot_json, status,
                            status_code, response_headers_json, response_body_preview,
                            response_body_truncated, duration_ms, error_message, executed_at
                     FROM api_request_histories WHERE id = ?1",
                    params![history_id],
                    map_history,
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => String::new(),
                    other => sql_error(other),
                });

            match result {
                Ok(inner) => inner.map(Some),
                Err(message) if message.is_empty() => Ok(None),
                Err(message) => Err(message),
            }
        })
    }
}

/// 请求快照脱敏：认证类请求头与疑似敏感的键值全部替换为占位值
pub fn redact_snapshot(snapshot: &RequestSnapshot) -> RequestSnapshot {
    RequestSnapshot {
        headers: redact_rows(&snapshot.headers),
        query: redact_rows(&snapshot.query),
        ..snapshot.clone()
    }
}

pub fn redact_rows(rows: &[KeyValueRow]) -> Vec<KeyValueRow> {
    rows.iter()
        .map(|row| {
            if is_sensitive_key(&row.key) {
                KeyValueRow {
                    value: REDACTED.to_string(),
                    ..row.clone()
                }
            } else {
                row.clone()
            }
        })
        .collect()
}

pub fn is_sensitive_key(key: &str) -> bool {
    let lower = key.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    SENSITIVE_HEADER_HINTS
        .iter()
        .any(|hint| lower == *hint || lower.contains(hint))
}

/// 历史响应体上限截断，按字符边界切分避免破坏 UTF-8
fn truncate_body(body: &str) -> (String, bool) {
    if body.len() <= MAX_HISTORY_BODY_BYTES {
        return (body.to_string(), false);
    }

    let mut end = MAX_HISTORY_BODY_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }

    (body[..end].to_string(), true)
}

fn sql_error(error: rusqlite::Error) -> String {
    format!("数据库操作失败: {error}")
}

fn map_history(row: &Row<'_>) -> rusqlite::Result<Result<ApiRequestHistory, String>> {
    let snapshot_json: String = row.get(3)?;
    let status_text: String = row.get(4)?;
    let status_code: Option<i64> = row.get(5)?;
    let headers_json: String = row.get(6)?;
    let truncated: i64 = row.get(8)?;
    let duration_ms: i64 = row.get(9)?;

    let base = ApiRequestHistory {
        id: row.get(0)?,
        request_id: row.get(1)?,
        environment_name: row.get(2)?,
        request_snapshot: RequestSnapshot::default(),
        status: ExecutionStatus::Success,
        status_code: status_code.map(|code| code.clamp(0, u16::MAX as i64) as u16),
        response_headers: Vec::new(),
        response_body_preview: row.get(7)?,
        response_body_truncated: truncated != 0,
        duration_ms: duration_ms.max(0) as u64,
        error_message: row.get(10)?,
        executed_at: row.get(11)?,
    };

    Ok((|| {
        Ok(ApiRequestHistory {
            request_snapshot: serde_json::from_str(&snapshot_json)
                .map_err(|error| format!("解析请求快照失败: {error}"))?,
            status: ExecutionStatus::parse(&status_text)?,
            response_headers: if headers_json.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&headers_json)
                    .map_err(|error| format!("解析响应头失败: {error}"))?
            },
            ..base
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::api_group_repo::ApiGroupRepository;
    use crate::db::repositories::api_project_repo::ApiProjectRepository;
    use crate::db::repositories::api_request_repo::ApiRequestRepository;
    use crate::models::api_client::{BodyKind, RequestBody};

    fn seeded() -> (ApiClientDatabase, String) {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let request_id = {
            let project_repo = ApiProjectRepository::new(&database);
            let project = project_repo.create("订单系统", "").expect("project");
            let group = ApiGroupRepository::new(&database)
                .list(&project.id)
                .expect("groups")[0]
                .clone();
            ApiRequestRepository::new(&database)
                .create(&group.id, "新增订单")
                .expect("request")
                .id
        };
        (database, request_id)
    }

    fn snapshot() -> RequestSnapshot {
        RequestSnapshot {
            method: "POST".to_string(),
            url: "https://test.example.com/orders".to_string(),
            query: vec![KeyValueRow::new("token", "plain-secret")],
            headers: vec![
                KeyValueRow::new("Authorization", "Bearer real-token"),
                KeyValueRow::new("Content-Type", "application/json"),
            ],
            body: RequestBody {
                kind: BodyKind::Json,
                text: "{\"items\":3}".to_string(),
                form: Vec::new(),
            },
            timeout_ms: 30_000,
            environment_name: Some("测试环境".to_string()),
        }
    }

    #[test]
    fn append_redacts_auth_headers_and_sensitive_query() {
        let (database, request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        let saved = repo
            .append(&ApiRequestHistory {
                request_id: request_id.clone(),
                environment_name: Some("测试环境".to_string()),
                request_snapshot: snapshot(),
                status: ExecutionStatus::Success,
                status_code: Some(200),
                response_headers: vec![
                    KeyValueRow::new("Set-Cookie", "session=abc"),
                    KeyValueRow::new("Content-Type", "application/json"),
                ],
                response_body_preview: "{\"id\":1}".to_string(),
                duration_ms: 42,
                ..Default::default()
            })
            .expect("history should be written");

        let auth = saved
            .request_snapshot
            .headers
            .iter()
            .find(|row| row.key == "Authorization")
            .expect("authorization header should be kept as a name");
        assert_eq!(auth.value, REDACTED, "credential value must not be stored");

        let content_type = saved
            .request_snapshot
            .headers
            .iter()
            .find(|row| row.key == "Content-Type")
            .expect("content type stays");
        assert_eq!(content_type.value, "application/json");

        assert_eq!(saved.request_snapshot.query[0].value, REDACTED);
        assert_eq!(
            saved
                .response_headers
                .iter()
                .find(|row| row.key == "Set-Cookie")
                .expect("set-cookie row")
                .value,
            REDACTED
        );

        let reloaded = repo.list(&request_id).expect("history should load");
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].status, ExecutionStatus::Success);
        assert_eq!(reloaded[0].status_code, Some(200));
        assert!(!reloaded[0]
            .request_snapshot
            .headers
            .iter()
            .any(|row| row.value.contains("real-token")));
    }

    #[test]
    fn append_truncates_response_body_beyond_history_limit() {
        let (database, request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        let oversized = "a".repeat(MAX_HISTORY_BODY_BYTES + 1024);
        let saved = repo
            .append(&ApiRequestHistory {
                request_id,
                request_snapshot: snapshot(),
                status: ExecutionStatus::Success,
                status_code: Some(200),
                response_body_preview: oversized,
                ..Default::default()
            })
            .expect("history should be written");

        assert!(
            saved.response_body_truncated,
            "oversized body must be flagged"
        );
        assert_eq!(saved.response_body_preview.len(), MAX_HISTORY_BODY_BYTES);
    }

    #[test]
    fn truncate_body_keeps_utf8_boundaries() {
        let body = "中".repeat(MAX_HISTORY_BODY_BYTES);
        let (truncated, was_truncated) = truncate_body(&body);
        assert!(was_truncated);
        assert!(truncated.len() <= MAX_HISTORY_BODY_BYTES);
        assert!(truncated.chars().all(|character| character == '中'));
    }

    #[test]
    fn append_keeps_only_recent_hundred_entries() {
        let (database, request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        for index in 0..(MAX_HISTORY_PER_REQUEST + 5) {
            repo.append(&ApiRequestHistory {
                request_id: request_id.clone(),
                request_snapshot: snapshot(),
                status: ExecutionStatus::Success,
                status_code: Some(200),
                response_body_preview: format!("{{\"index\":{index}}}"),
                executed_at: 1_000 + index as i64,
                ..Default::default()
            })
            .expect("history should be written");
        }

        let stored = repo.list(&request_id).expect("history should load");
        assert_eq!(stored.len(), MAX_HISTORY_PER_REQUEST);
        assert_eq!(
            stored[0].response_body_preview,
            format!("{{\"index\":{}}}", MAX_HISTORY_PER_REQUEST + 4),
            "newest entry must stay"
        );
        assert!(
            !stored
                .iter()
                .any(|entry| entry.response_body_preview == "{\"index\":0}"),
            "oldest entries must be pruned"
        );
    }

    #[test]
    fn append_stores_error_states_without_status_code() {
        let (database, request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        for status in [
            ExecutionStatus::NetworkError,
            ExecutionStatus::Timeout,
            ExecutionStatus::Cancelled,
        ] {
            repo.append(&ApiRequestHistory {
                request_id: request_id.clone(),
                request_snapshot: snapshot(),
                status,
                status_code: None,
                error_message: Some("连接失败".to_string()),
                ..Default::default()
            })
            .expect("error history should be written");
        }

        let stored = repo.list(&request_id).expect("history should load");
        assert_eq!(stored.len(), 3);
        assert!(stored.iter().all(|entry| entry.status_code.is_none()));
        assert!(stored
            .iter()
            .all(|entry| entry.error_message.as_deref() == Some("连接失败")));
    }

    #[test]
    fn append_for_unknown_request_is_rejected_without_losing_response() {
        let (database, _request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        let error = repo
            .append(&ApiRequestHistory {
                request_id: "missing".to_string(),
                request_snapshot: snapshot(),
                status: ExecutionStatus::Success,
                ..Default::default()
            })
            .expect_err("history for unknown request must fail");

        assert!(error.contains("数据库操作失败"), "got {error}");
    }

    #[test]
    fn clear_removes_all_history_for_request() {
        let (database, request_id) = seeded();
        let repo = ApiHistoryRepository::new(&database);

        repo.append(&ApiRequestHistory {
            request_id: request_id.clone(),
            request_snapshot: snapshot(),
            status: ExecutionStatus::Success,
            ..Default::default()
        })
        .expect("history should be written");

        assert_eq!(repo.clear(&request_id).expect("clear should succeed"), 1);
        assert!(repo
            .list(&request_id)
            .expect("history should load")
            .is_empty());
    }

    #[test]
    fn is_sensitive_key_matches_common_credential_names() {
        for key in [
            "Authorization",
            "cookie",
            "X-API-Key",
            "x-auth-token",
            "refresh_token",
            "USER_PASSWORD",
        ] {
            assert!(
                is_sensitive_key(key),
                "{key} should be treated as sensitive"
            );
        }

        for key in ["Content-Type", "Accept", "page", "userId"] {
            assert!(!is_sensitive_key(key), "{key} should not be redacted");
        }
    }
}

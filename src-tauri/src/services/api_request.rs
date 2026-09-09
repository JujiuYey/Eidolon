//! 接口请求执行编排：解析请求所属环境、构建请求、执行 HTTP 调用、
//! 把结果落库为执行快照；历史写入失败时透出 `ExecutionResult::history_error`，
//! 不吞掉已经收到的响应。

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_history::ApiHistoryRepository;
use crate::db::repositories::api_request_repo::ApiRequestRepository;
use crate::models::api_client::{ApiRequestHistory, ExecutionResult, RequestSnapshot};
use crate::services::api_http::{new_execution_id, prepare_request, ApiHttpClient};

/// 发送一次请求。快照由调用方冻结，本函数不会写回请求定义。
pub async fn execute(
    database: &ApiClientDatabase,
    http_client: &ApiHttpClient,
    request_id: &str,
    environment_id: Option<&str>,
    snapshot: RequestSnapshot,
    execution_id: Option<&str>,
) -> Result<ExecutionResult, String> {
    let environment =
        ApiRequestRepository::new(database).resolve_environment(request_id, environment_id)?;

    let prepared = prepare_request(&snapshot, environment.as_ref())?;

    let execution_id = execution_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(new_execution_id);

    let mut result = http_client.send(&execution_id, &prepared).await?;

    let environment_name = environment
        .as_ref()
        .map(|environment| environment.name.clone());

    // 历史写入失败不能吞掉已经收到的响应
    let history = ApiHistoryRepository::new(database).append(&ApiRequestHistory {
        request_id: request_id.to_string(),
        environment_name: environment_name.clone(),
        request_snapshot: RequestSnapshot {
            environment_name,
            ..snapshot
        },
        status: result.status,
        status_code: result.status_code,
        response_headers: result.response_headers.clone(),
        response_body_preview: result.body_text.clone(),
        response_body_truncated: result.is_oversized,
        duration_ms: result.duration_ms,
        error_message: result.error_message.clone(),
        ..Default::default()
    });

    match history {
        Ok(saved) => result.history_id = Some(saved.id),
        Err(error) => result.history_error = Some(format!("响应已收到，但历史未保存: {error}")),
    }

    Ok(result)
}

/// 取消等待。服务端可能已经执行，不显示为“已回滚”。
pub fn cancel(http_client: &ApiHttpClient, execution_id: &str) -> Result<bool, String> {
    http_client.registry().cancel(execution_id)
}

/// 仅校验请求而不发送：用于在 UI 上提前提示缺失变量和的不合法 JSON。
/// 返回 [`PreparedRequestPreview`]，不含完整 body（只给字节数）。
pub fn preview(
    database: &ApiClientDatabase,
    request_id: &str,
    environment_id: Option<&str>,
    snapshot: &RequestSnapshot,
) -> Result<PreparedRequestPreview, String> {
    let environment =
        ApiRequestRepository::new(database).resolve_environment(request_id, environment_id)?;
    let prepared = prepare_request(snapshot, environment.as_ref())?;

    Ok(PreparedRequestPreview {
        method: prepared.method,
        url: prepared.url,
        headers: prepared.headers,
        body_size_bytes: prepared.body.as_ref().map(|body| body.len()).unwrap_or(0),
        environment_name: environment.map(|environment| environment.name),
    })
}

/// 校验预览的对外 DTO。字段命名与前端 `TauriPreparedRequestPreview` 一致
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreparedRequestPreview {
    pub method: String,
    pub url: String,
    pub headers: Vec<crate::models::api_client::KeyValueRow>,
    pub body_size_bytes: usize,
    pub environment_name: Option<String>,
}

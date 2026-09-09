use crate::db::api_client::ApiClientDatabase;
use crate::models::api_client::{ExecutionResult, RequestSnapshot};
use crate::services::api_http::ApiHttpClient;
use crate::services::api_request::{cancel, execute, preview, PreparedRequestPreview};

/// 发送一次请求。快照由前端冻结，发送不会写回请求定义。
#[tauri::command]
pub async fn send_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    http_client: tauri::State<'_, ApiHttpClient>,
    request_id: String,
    environment_id: Option<String>,
    snapshot: RequestSnapshot,
    execution_id: Option<String>,
) -> Result<ExecutionResult, String> {
    execute(
        database.inner(),
        http_client.inner(),
        &request_id,
        environment_id.as_deref(),
        snapshot,
        execution_id.as_deref(),
    )
    .await
}

/// 取消等待。服务端可能已经执行，不显示为“已回滚”。
#[tauri::command]
pub fn cancel_api_request(
    http_client: tauri::State<'_, ApiHttpClient>,
    execution_id: String,
) -> Result<bool, String> {
    cancel(http_client.inner(), &execution_id)
}

/// 校验但不发送：用于在 UI 上提前提示缺失变量和的不合法 JSON。
#[tauri::command]
pub fn preview_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
    environment_id: Option<String>,
    snapshot: RequestSnapshot,
) -> Result<PreparedRequestPreview, String> {
    preview(
        database.inner(),
        &request_id,
        environment_id.as_deref(),
        &snapshot,
    )
}

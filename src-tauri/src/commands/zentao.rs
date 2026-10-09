use serde::{Deserialize, Serialize};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::zentao_account_repo::ZentaoAccountRepository;
use crate::models::zentao::{ZentaoAccount, ZentaoMyWork};
use crate::services::zentao::ZentaoClient;

// ===== 禅道账户 =====

#[tauri::command]
pub fn list_zentao_accounts(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<ZentaoAccount>, String> {
    ZentaoAccountRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_zentao_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account: ZentaoAccount,
) -> Result<ZentaoAccount, String> {
    ZentaoAccountRepository::new(&database).upsert(&account)
}

#[tauri::command]
pub fn delete_zentao_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account_id: String,
) -> Result<String, String> {
    ZentaoAccountRepository::new(&database).delete(&account_id)
}

// ===== 连接测试与数据拉取 =====

#[derive(Debug, Deserialize)]
pub struct TestZentaoConnectionRequest {
    pub base_url: String,
    pub account: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TestZentaoConnectionResponse {
    pub success: bool,
    pub message: String,
}

/// 用填写的凭据直接登录禅道验证连通性，不依赖已保存的账户
#[tauri::command]
pub async fn test_zentao_connection(
    client: tauri::State<'_, ZentaoClient>,
    request: TestZentaoConnectionRequest,
) -> Result<TestZentaoConnectionResponse, String> {
    client
        .test_connection(&request.base_url, &request.account, &request.password)
        .await?;

    Ok(TestZentaoConnectionResponse {
        success: true,
        message: "禅道登录成功".to_string(),
    })
}

/// 拉取"我的工作台"：进行中任务 + 未关闭 Bug（客户端过滤）
#[tauri::command]
pub async fn fetch_zentao_my_work(
    database: tauri::State<'_, ApiClientDatabase>,
    client: tauri::State<'_, ZentaoClient>,
    account_id: String,
) -> Result<ZentaoMyWork, String> {
    client.fetch_my_work(database.inner(), &account_id).await
}

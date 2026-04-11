use rig::{client::CompletionClient, completion::CompletionModel, providers::openai::Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn test_ai_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let api_key = request.api_key.trim();
    let base_url = request.base_url.trim();
    let model = request.model.trim();

    // 使用 chat/completions 兼容接口，和前端 ChatOpenAI 的调用方式保持一致。
    // rig-core 0.33 默认会走 /responses，很多 OpenAI 兼容供应商并不支持该端点。
    let client = Client::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| format!("创建客户端失败: {}", e))?
        .completions_api();

    // 发送简单测试请求
    let _response = client
        .completion_model(model)
        .completion_request("Hi")
        .send()
        .await
        .map_err(|e| format_test_connection_error(&e.to_string(), base_url))?;

    log::info!("测试连接成功");

    Ok(TestConnectionResponse {
        success: true,
        message: "连接成功!".to_string(),
    })
}

fn format_test_connection_error(error: &str, base_url: &str) -> String {
    if error.contains("404") {
        return format!(
            "测试请求失败: {}。请确认 Base URL 是否正确，并且该服务支持 OpenAI Chat Completions 兼容接口；当前地址: {}",
            error, base_url
        );
    }

    format!("测试请求失败: {}", error)
}

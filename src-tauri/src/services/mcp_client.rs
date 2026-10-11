//! 通用 MCP 工具调用层：在 discovery（发现）之外补上"执行"。
//!
//! 连接模式与 `discover_service` 一致：单次调用现连现断；
//! 长流程（如数据字典采集循环）用 `connect_service` 自行持有连接，
//! 结束时调用 `cancel()` 释放。

use std::process::Stdio;
use std::time::Duration;

use rmcp::{
    model::{CallToolRequestParams, ClientInfo},
    service::RunningService,
    transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use serde_json::Value;
use tokio::process::Command;
use tokio::time::timeout;

use super::mcp_service::{append_stderr, parse_args, parse_env};
use crate::models::mcp_service::{McpService, McpTransportType};

pub type McpClientHandle = RunningService<RoleClient, ClientInfo>;

/// 连接到 MCP 服务；调用方负责在结束时 `cancel()`
pub async fn connect_service(service: &McpService) -> Result<McpClientHandle, String> {
    let timeout_seconds = service.timeout_seconds.max(1);
    timeout(Duration::from_secs(timeout_seconds), async {
        match service.transport_type {
            McpTransportType::Stdio => connect_stdio(service).await,
            McpTransportType::StreamableHttp => connect_streamable_http(service).await,
        }
    })
    .await
    .map_err(|_| format!("连接超时（>{timeout_seconds} 秒）"))?
}

/// 单次工具调用：现连现调现断
pub async fn call_tool_once(
    service: &McpService,
    tool_name: &str,
    arguments: Value,
) -> Result<String, String> {
    let client = connect_service(service).await?;
    let result = call_tool(client.peer(), tool_name, arguments).await;
    let _ = client.cancel().await;
    result
}

/// 在已持有的连接上调用工具；错误结果（is_error）转为 Err 并携带错误文本
pub async fn call_tool(
    peer: &rmcp::service::Peer<RoleClient>,
    tool_name: &str,
    arguments: Value,
) -> Result<String, String> {
    let object = match arguments {
        Value::Object(map) => map,
        other => {
            return Err(format!(
                "工具 {tool_name} 的参数必须是 JSON 对象，收到: {other}"
            ))
        }
    };

    let result = peer
        .call_tool(CallToolRequestParams {
            name: tool_name.to_string().into(),
            arguments: Some(object),
            meta: None,
            task: None,
        })
        .await
        .map_err(|error| format!("调用工具 {tool_name} 失败: {error}"))?;

    let mut text = String::new();
    for content in &result.content {
        if let Some(raw) = content.as_text() {
            text.push_str(&raw.text);
        }
    }

    if result.is_error.unwrap_or(false) {
        return Err(if text.is_empty() {
            format!("工具 {tool_name} 返回错误，但没有错误详情")
        } else {
            text
        });
    }

    Ok(text)
}

async fn connect_stdio(service: &McpService) -> Result<McpClientHandle, String> {
    let args = parse_args(&service.args);
    let env = parse_env(&service.env)?;

    let (transport, stderr) =
        TokioChildProcess::builder(Command::new(service.command.trim()).configure(|command| {
            command.args(&args);
            command.envs(&env);
        }))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("启动 MCP 进程失败: {error}"))?;

    match ClientInfo::default().serve(transport).await {
        Ok(client) => Ok(client),
        Err(error) => {
            Err(append_stderr(format!("连接 MCP 服务失败: {error}"), stderr).await)
        }
    }
}

async fn connect_streamable_http(service: &McpService) -> Result<McpClientHandle, String> {
    let transport = StreamableHttpClientTransport::from_uri(service.url.trim().to_string());
    ClientInfo::default()
        .serve(transport)
        .await
        .map_err(|error| format!("连接 MCP 服务失败: {error}"))
}

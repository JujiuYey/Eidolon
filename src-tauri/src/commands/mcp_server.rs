//! 对外 MCP 服务命令：设置读写、启停、令牌再生。

use std::sync::Arc;

use tauri::State;

use crate::db::LocalJsonStore;
use crate::models::mcp_server::McpServerInfo;
use crate::services::mcp_server::{self, McpServerHandle};

/// 查询设置与运行状态（设置页展示 + 生成客户端配置）
#[tauri::command]
pub async fn get_mcp_server_info(
    store: State<'_, LocalJsonStore>,
    handle: State<'_, McpServerHandle>,
) -> Result<McpServerInfo, String> {
    let settings = mcp_server::get_settings(&store);
    let running = handle.is_running().await;
    let active_port = handle.active_port().await;
    Ok(McpServerInfo {
        enabled: settings.enabled,
        port: settings.port,
        token: settings.token,
        running,
        active_port,
    })
}

/// 更新设置（enabled/port）。服务运行中或启用状态变化时自动重启。
#[tauri::command]
pub async fn update_mcp_server_settings(
    store: State<'_, LocalJsonStore>,
    handle: State<'_, McpServerHandle>,
    enabled: Option<bool>,
    port: Option<u16>,
) -> Result<McpServerInfo, String> {
    let mut settings = mcp_server::load_or_init(&store)?;
    if let Some(enabled) = enabled {
        settings.enabled = enabled;
    }
    if let Some(port) = port {
        settings.port = port;
    }
    let settings = mcp_server::save_settings(&store, settings)?;

    // 启用：确保在运行（start 内部会先停旧实例，端口变更随之生效）；关闭：停止
    if settings.enabled {
        let dedicated = dedicated_store(&store)?;
        handle.start(&settings, dedicated).await?;
    } else {
        handle.stop().await;
    }

    get_info(&store, &handle).await
}

/// 重新生成访问令牌（服务运行中会重启以应用）
#[tauri::command]
pub async fn regenerate_mcp_server_token(
    store: State<'_, LocalJsonStore>,
    handle: State<'_, McpServerHandle>,
) -> Result<McpServerInfo, String> {
    let mut settings = mcp_server::load_or_init(&store)?;
    settings.token = mcp_server::generate_token();
    let settings = mcp_server::save_settings(&store, settings)?;

    if handle.is_running().await {
        let dedicated = dedicated_store(&store)?;
        handle.start(&settings, dedicated).await?;
    }

    get_info(&store, &handle).await
}

async fn get_info(
    store: &LocalJsonStore,
    handle: &McpServerHandle,
) -> Result<McpServerInfo, String> {
    let settings = mcp_server::get_settings(store);
    Ok(McpServerInfo {
        enabled: settings.enabled,
        port: settings.port,
        token: settings.token,
        running: handle.is_running().await,
        active_port: handle.active_port().await,
    })
}

/// 为 MCP 服务任务创建独立的存储实例（LocalJsonStore 只是路径包装，
/// 无需共享同一实例）
fn dedicated_store(store: &LocalJsonStore) -> Result<Arc<LocalJsonStore>, String> {
    Ok(Arc::new(LocalJsonStore::new(
        store.data_dir().to_path_buf(),
    )?))
}

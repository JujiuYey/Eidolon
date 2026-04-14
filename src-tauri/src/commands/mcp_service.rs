use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::mcp_service::McpServiceRepository;
use crate::models::mcp_service::{McpService, McpServiceDiscovery};
use crate::services::mcp_service::discover_service;

#[tauri::command]
pub fn list_mcp_services(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<McpService>, String> {
    let repo = McpServiceRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn upsert_mcp_service(
    store: tauri::State<'_, LocalJsonStore>,
    service: McpService,
) -> Result<String, String> {
    let repo = McpServiceRepository::new(&store);
    repo.upsert(&service)
}

#[tauri::command]
pub fn delete_mcp_service(
    store: tauri::State<'_, LocalJsonStore>,
    service_id: String,
) -> Result<String, String> {
    let repo = McpServiceRepository::new(&store);
    repo.delete(&service_id)
}

#[tauri::command]
pub async fn discover_mcp_service(
    service: McpService,
) -> Result<McpServiceDiscovery, String> {
    discover_service(&service).await
}

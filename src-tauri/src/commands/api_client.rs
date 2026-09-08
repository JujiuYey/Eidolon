use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client::{ApiClientRepository, DeletionSummary};
use crate::db::repositories::api_history::ApiHistoryRepository;
use crate::models::api_client::{
    ApiEnvironment, ApiGroup, ApiProject, ApiRequest, ApiRequestHistory,
};

#[tauri::command]
pub fn list_api_projects(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<ApiProject>, String> {
    ApiClientRepository::new(&database).list_projects()
}

#[tauri::command]
pub fn create_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    name: String,
    description: Option<String>,
) -> Result<ApiProject, String> {
    ApiClientRepository::new(&database)
        .create_project(&name, description.unwrap_or_default().as_str())
}

#[tauri::command]
pub fn rename_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    name: String,
) -> Result<ApiProject, String> {
    ApiClientRepository::new(&database).rename_project(&project_id, &name)
}

#[tauri::command]
pub fn preview_api_project_deletion(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<DeletionSummary, String> {
    ApiClientRepository::new(&database).preview_project_deletion(&project_id)
}

#[tauri::command]
pub fn delete_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<DeletionSummary, String> {
    ApiClientRepository::new(&database).delete_project(&project_id)
}

#[tauri::command]
pub fn list_api_groups(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiGroup>, String> {
    ApiClientRepository::new(&database).list_groups(&project_id)
}

#[tauri::command]
pub fn create_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    name: String,
) -> Result<ApiGroup, String> {
    ApiClientRepository::new(&database).create_group(&project_id, &name)
}

#[tauri::command]
pub fn rename_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    name: String,
) -> Result<ApiGroup, String> {
    ApiClientRepository::new(&database).rename_group(&group_id, &name)
}

#[tauri::command]
pub fn delete_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
) -> Result<DeletionSummary, String> {
    ApiClientRepository::new(&database).delete_group(&group_id)
}

#[tauri::command]
pub fn reorder_api_groups(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    group_ids: Vec<String>,
) -> Result<Vec<ApiGroup>, String> {
    ApiClientRepository::new(&database).reorder_groups(&project_id, &group_ids)
}

#[tauri::command]
pub fn list_api_requests(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiRequest>, String> {
    ApiClientRepository::new(&database).list_requests(&project_id)
}

#[tauri::command]
pub fn get_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<Option<ApiRequest>, String> {
    ApiClientRepository::new(&database).get_request(&request_id)
}

#[tauri::command]
pub fn create_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    name: String,
) -> Result<ApiRequest, String> {
    ApiClientRepository::new(&database).create_request(&group_id, &name)
}

/// 保存请求定义。发送未保存的草稿不会走这个 command。
#[tauri::command]
pub fn update_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request: ApiRequest,
) -> Result<ApiRequest, String> {
    ApiClientRepository::new(&database).update_request(&request)
}

#[tauri::command]
pub fn duplicate_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<ApiRequest, String> {
    ApiClientRepository::new(&database).duplicate_request(&request_id)
}

#[tauri::command]
pub fn move_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
    target_group_id: String,
) -> Result<ApiRequest, String> {
    ApiClientRepository::new(&database).move_request(&request_id, &target_group_id)
}

#[tauri::command]
pub fn delete_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<DeletionSummary, String> {
    ApiClientRepository::new(&database).delete_request(&request_id)
}

#[tauri::command]
pub fn reorder_api_requests(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    request_ids: Vec<String>,
) -> Result<Vec<ApiRequest>, String> {
    ApiClientRepository::new(&database).reorder_requests(&group_id, &request_ids)
}

#[tauri::command]
pub fn list_api_environments(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiEnvironment>, String> {
    ApiClientRepository::new(&database).list_environments(&project_id)
}

#[tauri::command]
pub fn upsert_api_environment(
    database: tauri::State<'_, ApiClientDatabase>,
    environment: ApiEnvironment,
) -> Result<ApiEnvironment, String> {
    ApiClientRepository::new(&database).upsert_environment(&environment)
}

#[tauri::command]
pub fn delete_api_environment(
    database: tauri::State<'_, ApiClientDatabase>,
    environment_id: String,
) -> Result<String, String> {
    ApiClientRepository::new(&database).delete_environment(&environment_id)
}

#[tauri::command]
pub fn list_api_request_histories(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<Vec<ApiRequestHistory>, String> {
    ApiHistoryRepository::new(&database).list(&request_id)
}

#[tauri::command]
pub fn get_api_request_history(
    database: tauri::State<'_, ApiClientDatabase>,
    history_id: String,
) -> Result<Option<ApiRequestHistory>, String> {
    ApiHistoryRepository::new(&database).get(&history_id)
}

#[tauri::command]
pub fn clear_api_request_histories(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<usize, String> {
    ApiHistoryRepository::new(&database).clear(&request_id)
}

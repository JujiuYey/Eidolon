use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::DeletionSummary;
use crate::db::repositories::api_environment_repo::ApiEnvironmentRepository;
use crate::db::repositories::api_group_repo::ApiGroupRepository;
use crate::db::repositories::api_history::ApiHistoryRepository;
use crate::db::repositories::api_project_repo::ApiProjectRepository;
use crate::db::repositories::api_request_repo::ApiRequestRepository;
use crate::models::api_client::{
    ApiEnvironment, ApiGroup, ApiProject, ApiRequest, ApiRequestHistory,
};

#[tauri::command]
pub fn list_api_projects(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<ApiProject>, String> {
    ApiProjectRepository::new(&database).list()
}

#[tauri::command]
pub fn create_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    name: String,
    description: Option<String>,
) -> Result<ApiProject, String> {
    ApiProjectRepository::new(&database).create(&name, description.unwrap_or_default().as_str())
}

#[tauri::command]
pub fn rename_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    name: String,
) -> Result<ApiProject, String> {
    ApiProjectRepository::new(&database).rename(&project_id, &name)
}

#[tauri::command]
pub fn update_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    name: String,
    description: String,
) -> Result<ApiProject, String> {
    ApiProjectRepository::new(&database).update(&project_id, &name, &description)
}

#[tauri::command]
pub fn preview_api_project_deletion(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<DeletionSummary, String> {
    ApiProjectRepository::new(&database).preview_deletion(&project_id)
}

#[tauri::command]
pub fn delete_api_project(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<DeletionSummary, String> {
    ApiProjectRepository::new(&database).delete(&project_id)
}

#[tauri::command]
pub fn list_api_groups(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiGroup>, String> {
    ApiGroupRepository::new(&database).list(&project_id)
}

#[tauri::command]
pub fn create_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    name: String,
) -> Result<ApiGroup, String> {
    ApiGroupRepository::new(&database).create(&project_id, &name)
}

#[tauri::command]
pub fn rename_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    name: String,
) -> Result<ApiGroup, String> {
    ApiGroupRepository::new(&database).rename(&group_id, &name)
}

#[tauri::command]
pub fn delete_api_group(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
) -> Result<DeletionSummary, String> {
    ApiGroupRepository::new(&database).delete(&group_id)
}

#[tauri::command]
pub fn reorder_api_groups(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    group_ids: Vec<String>,
) -> Result<Vec<ApiGroup>, String> {
    ApiGroupRepository::new(&database).reorder(&project_id, &group_ids)
}

#[tauri::command]
pub fn list_api_requests(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiRequest>, String> {
    ApiRequestRepository::new(&database).list(&project_id)
}

#[tauri::command]
pub fn get_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<Option<ApiRequest>, String> {
    ApiRequestRepository::new(&database).get(&request_id)
}

#[tauri::command]
pub fn create_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    name: String,
) -> Result<ApiRequest, String> {
    ApiRequestRepository::new(&database).create(&group_id, &name)
}

/// 保存请求定义。发送未保存的草稿不会走这个 command。
#[tauri::command]
pub fn update_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request: ApiRequest,
) -> Result<ApiRequest, String> {
    ApiRequestRepository::new(&database).update(&request)
}

#[tauri::command]
pub fn duplicate_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<ApiRequest, String> {
    ApiRequestRepository::new(&database).duplicate(&request_id)
}

#[tauri::command]
pub fn move_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
    target_group_id: String,
) -> Result<ApiRequest, String> {
    ApiRequestRepository::new(&database).move_to_group(&request_id, &target_group_id)
}

#[tauri::command]
pub fn delete_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
) -> Result<DeletionSummary, String> {
    ApiRequestRepository::new(&database).delete(&request_id)
}

#[tauri::command]
pub fn reorder_api_requests(
    database: tauri::State<'_, ApiClientDatabase>,
    group_id: String,
    request_ids: Vec<String>,
) -> Result<Vec<ApiRequest>, String> {
    ApiRequestRepository::new(&database).reorder(&group_id, &request_ids)
}

#[tauri::command]
pub fn list_api_environments(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiEnvironment>, String> {
    ApiEnvironmentRepository::new(&database).list(&project_id)
}

#[tauri::command]
pub fn upsert_api_environment(
    database: tauri::State<'_, ApiClientDatabase>,
    environment: ApiEnvironment,
) -> Result<ApiEnvironment, String> {
    ApiEnvironmentRepository::new(&database).upsert(&environment)
}

#[tauri::command]
pub fn delete_api_environment(
    database: tauri::State<'_, ApiClientDatabase>,
    environment_id: String,
) -> Result<String, String> {
    ApiEnvironmentRepository::new(&database).delete(&environment_id)
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

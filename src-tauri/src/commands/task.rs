use crate::db::api_client::ApiClientDatabase;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::task_repo::TaskRepository;
use crate::models::task::{Task, TaskDraft, TaskPlan};
use crate::services::task_ai;

#[tauri::command]
pub fn list_tasks(database: tauri::State<'_, ApiClientDatabase>) -> Result<Vec<Task>, String> {
    TaskRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_task(
    database: tauri::State<'_, ApiClientDatabase>,
    task: Task,
) -> Result<Task, String> {
    TaskRepository::new(&database).upsert(&task)
}

#[tauri::command]
pub fn delete_task(
    database: tauri::State<'_, ApiClientDatabase>,
    task_id: String,
) -> Result<String, String> {
    TaskRepository::new(&database).delete(&task_id)
}

/// AI：把一句话整理成结构化任务草稿
#[tauri::command]
pub async fn parse_task_text(
    store: tauri::State<'_, LocalJsonStore>,
    input: String,
) -> Result<TaskDraft, String> {
    task_ai::parse_task_text(&store, &input).await
}

/// AI：把未排期任务排成结构化时间块（前端过目后应用）
#[tauri::command]
pub async fn suggest_task_plan(
    store: tauri::State<'_, LocalJsonStore>,
    tasks: Vec<Task>,
) -> Result<TaskPlan, String> {
    task_ai::suggest_task_plan(&store, &tasks).await
}

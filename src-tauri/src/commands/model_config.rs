use crate::db::mongodb::repositories::model_config::ModelConfigRepository;
use crate::db::mongodb::AppMongoDb;
use crate::models::model_config::ModelProviderSettingDoc;

#[tauri::command]
pub async fn list_model_configs(
    mongo: tauri::State<'_, AppMongoDb>,
) -> Result<Vec<ModelProviderSettingDoc>, String> {
    let repo = ModelConfigRepository::new(&mongo);
    repo.list().await
}

#[tauri::command]
pub async fn create_model_config(
    mongo: tauri::State<'_, AppMongoDb>,
    config: ModelProviderSettingDoc,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&mongo);
    repo.create(&config).await
}

#[tauri::command]
pub async fn update_model_config(
    mongo: tauri::State<'_, AppMongoDb>,
    config: ModelProviderSettingDoc,
) -> Result<(), String> {
    let repo = ModelConfigRepository::new(&mongo);
    repo.update(&config).await
}

#[tauri::command]
pub async fn delete_model_config(
    mongo: tauri::State<'_, AppMongoDb>,
    id: String,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&mongo);
    repo.delete(&id).await
}

#[tauri::command]
pub async fn set_default_config(
    mongo: tauri::State<'_, AppMongoDb>,
    id: String,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&mongo);
    repo.set_default(&id).await
}

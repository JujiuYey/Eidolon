use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::model_config::ModelConfigRepository;
use crate::models::model_config::ModelProviderSettingDoc;

#[tauri::command]
pub fn list_model_configs(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<ModelProviderSettingDoc>, String> {
    let repo = ModelConfigRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn create_model_config(
    store: tauri::State<'_, LocalJsonStore>,
    config: ModelProviderSettingDoc,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&store);
    repo.upsert(&config)
}

#[tauri::command]
pub fn update_model_config(
    store: tauri::State<'_, LocalJsonStore>,
    config: ModelProviderSettingDoc,
) -> Result<(), String> {
    let repo = ModelConfigRepository::new(&store);
    repo.update(&config)
}

#[tauri::command]
pub fn delete_model_config(
    store: tauri::State<'_, LocalJsonStore>,
    id: String,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&store);
    repo.delete(&id)
}

#[tauri::command]
pub fn set_default_config(
    store: tauri::State<'_, LocalJsonStore>,
    id: String,
) -> Result<String, String> {
    let repo = ModelConfigRepository::new(&store);
    repo.set_default(&id)
}

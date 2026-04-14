use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::model_config::{
    ProviderModelRepository, ProviderSettingRepository,
};
use crate::models::model_config::{ProviderModel, ProviderSetting};

#[tauri::command]
pub fn list_provider_settings(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<ProviderSetting>, String> {
    let repo = ProviderSettingRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn upsert_provider_setting(
    store: tauri::State<'_, LocalJsonStore>,
    setting: ProviderSetting,
) -> Result<String, String> {
    let repo = ProviderSettingRepository::new(&store);
    repo.upsert(&setting)
}

#[tauri::command]
pub fn delete_provider_setting(
    store: tauri::State<'_, LocalJsonStore>,
    provider_id: String,
) -> Result<String, String> {
    let repo = ProviderSettingRepository::new(&store);
    repo.delete(&provider_id)
}

#[tauri::command]
pub fn list_provider_models(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<ProviderModel>, String> {
    let repo = ProviderModelRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn replace_provider_models(
    store: tauri::State<'_, LocalJsonStore>,
    provider_id: String,
    models: Vec<ProviderModel>,
) -> Result<String, String> {
    let repo = ProviderModelRepository::new(&store);
    repo.replace_for_provider(&provider_id, &models)
}

#[tauri::command]
pub fn delete_provider_models(
    store: tauri::State<'_, LocalJsonStore>,
    provider_id: String,
) -> Result<String, String> {
    let repo = ProviderModelRepository::new(&store);
    repo.delete_by_provider(&provider_id)
}

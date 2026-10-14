use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::model_config::{ProviderModelRepository, ProviderSettingRepository};
use crate::models::model_config::{ProviderModel, ProviderSetting};
use crate::services::models_endpoint::{self, FetchedModel};

/// 拉取远端模型清单的入参；不落库，能力标记仍由用户确认后保存
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchProviderModelsRequest {
    pub provider_id: String,
    pub base_url: String,
    pub api_key: String,
}

#[tauri::command]
pub async fn fetch_provider_models(
    request: FetchProviderModelsRequest,
) -> Result<Vec<FetchedModel>, String> {
    let models =
        models_endpoint::fetch_models(&request.base_url, &request.api_key, &request.provider_id)
            .await?;
    log::info!(
        "拉取 {} 的模型列表成功，共 {} 个",
        request.provider_id,
        models.len()
    );
    Ok(models)
}

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

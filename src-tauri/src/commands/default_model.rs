use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::models::default_model::DefaultModelSetting;

#[tauri::command]
pub fn list_default_model_settings(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<DefaultModelSetting>, String> {
    let repo = DefaultModelSettingRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn upsert_default_model_setting(
    store: tauri::State<'_, LocalJsonStore>,
    setting: DefaultModelSetting,
) -> Result<String, String> {
    let repo = DefaultModelSettingRepository::new(&store);
    repo.upsert(&setting)
}

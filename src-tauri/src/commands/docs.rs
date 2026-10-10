use crate::db::local_store::LocalJsonStore;
use crate::models::docs::{DocsEntry, DocsSearchResult, DocsSettings};
use crate::services::docs;

#[tauri::command]
pub fn get_docs_settings(store: tauri::State<'_, LocalJsonStore>) -> Result<DocsSettings, String> {
    Ok(docs::get_settings(&store))
}

#[tauri::command]
pub fn upsert_docs_settings(
    store: tauri::State<'_, LocalJsonStore>,
    settings: DocsSettings,
) -> Result<DocsSettings, String> {
    docs::save_settings(&store, &settings)
}

#[tauri::command]
pub fn list_docs_entries(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
) -> Result<Vec<DocsEntry>, String> {
    docs::list_entries(&store, &root_id)
}

#[tauri::command]
pub fn read_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    path: String,
) -> Result<String, String> {
    docs::read_file(&store, &root_id, &path)
}

#[tauri::command]
pub fn save_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    path: String,
    content: String,
) -> Result<DocsEntry, String> {
    docs::save_file(&store, &root_id, &path, &content)
}

#[tauri::command]
pub fn create_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    dir_path: String,
    name: String,
) -> Result<DocsEntry, String> {
    docs::create_file(&store, &root_id, &dir_path, &name)
}

#[tauri::command]
pub fn create_docs_directory(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    parent_path: String,
    name: String,
) -> Result<DocsEntry, String> {
    docs::create_directory(&store, &root_id, &parent_path, &name)
}

#[tauri::command]
pub fn rename_docs_entry(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    path: String,
    new_name: String,
) -> Result<DocsEntry, String> {
    docs::rename_entry(&store, &root_id, &path, &new_name)
}

#[tauri::command]
pub fn delete_docs_entry(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    path: String,
) -> Result<String, String> {
    docs::delete_entry(&store, &root_id, &path)
}

#[tauri::command]
pub fn search_docs(
    store: tauri::State<'_, LocalJsonStore>,
    root_id: String,
    keyword: String,
) -> Result<Vec<DocsSearchResult>, String> {
    docs::search(&store, &root_id, &keyword)
}

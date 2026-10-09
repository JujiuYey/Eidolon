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
) -> Result<Vec<DocsEntry>, String> {
    docs::list_entries(&store)
}

#[tauri::command]
pub fn read_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    path: String,
) -> Result<String, String> {
    docs::read_file(&store, &path)
}

#[tauri::command]
pub fn save_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    path: String,
    content: String,
) -> Result<DocsEntry, String> {
    docs::save_file(&store, &path, &content)
}

#[tauri::command]
pub fn create_docs_file(
    store: tauri::State<'_, LocalJsonStore>,
    dir_path: String,
    name: String,
) -> Result<DocsEntry, String> {
    docs::create_file(&store, &dir_path, &name)
}

#[tauri::command]
pub fn create_docs_directory(
    store: tauri::State<'_, LocalJsonStore>,
    parent_path: String,
    name: String,
) -> Result<DocsEntry, String> {
    docs::create_directory(&store, &parent_path, &name)
}

#[tauri::command]
pub fn rename_docs_entry(
    store: tauri::State<'_, LocalJsonStore>,
    path: String,
    new_name: String,
) -> Result<DocsEntry, String> {
    docs::rename_entry(&store, &path, &new_name)
}

#[tauri::command]
pub fn delete_docs_entry(
    store: tauri::State<'_, LocalJsonStore>,
    path: String,
) -> Result<String, String> {
    docs::delete_entry(&store, &path)
}

#[tauri::command]
pub fn search_docs(
    store: tauri::State<'_, LocalJsonStore>,
    keyword: String,
) -> Result<Vec<DocsSearchResult>, String> {
    docs::search(&store, &keyword)
}

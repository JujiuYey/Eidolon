use crate::db::api_client::ApiClientDatabase;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::collect_repo::CollectItemRepository;
use crate::models::collect::{CollectItem, LinkMetadata};
use crate::services::{collect_ai, collect_fetch};

#[tauri::command]
pub fn list_collect_items(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<CollectItem>, String> {
    CollectItemRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_collect_item(
    database: tauri::State<'_, ApiClientDatabase>,
    item: CollectItem,
) -> Result<CollectItem, String> {
    CollectItemRepository::new(&database).upsert(&item)
}

#[tauri::command]
pub fn delete_collect_item(
    database: tauri::State<'_, ApiClientDatabase>,
    item_id: String,
) -> Result<String, String> {
    CollectItemRepository::new(&database).delete(&item_id)
}

/// 抓取链接的标题/摘要/favicon；保存链接后由前端异步调用回填
#[tauri::command]
pub async fn fetch_link_metadata(url: String) -> Result<LinkMetadata, String> {
    collect_fetch::fetch_link_metadata(&url).await
}

/// AI：总结单条链接（现场抓正文，不落库）
#[tauri::command]
pub async fn summarize_collect_link(
    store: tauri::State<'_, LocalJsonStore>,
    url: String,
) -> Result<String, String> {
    collect_ai::summarize_link(&store, &url).await
}

/// AI：基于全部收藏条目问答
#[tauri::command]
pub async fn ask_collect_items(
    store: tauri::State<'_, LocalJsonStore>,
    database: tauri::State<'_, ApiClientDatabase>,
    question: String,
) -> Result<String, String> {
    let items = CollectItemRepository::new(&database).list()?;
    if items.is_empty() {
        return Ok("收藏夹还是空的，先收藏一些链接或记事再来问吧。".to_string());
    }

    let context = collect_ai::build_context_lines(&items);
    collect_ai::answer_question(&store, &context, &question).await
}

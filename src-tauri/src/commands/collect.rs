use crate::db::api_client::ApiClientDatabase;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::collect_repo::CollectItemRepository;
use crate::models::collect::{CollectItem, LinkMetadata};
use crate::services::{collect_ai, collect_fetch};

#[tauri::command]
pub fn list_collect_items(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<CollectItem>, String> {
    let repo = CollectItemRepository::new(&database);
    // 惰性清理：超过保留期（30 天）的回收站条目在拉取时物理清除
    repo.purge_expired(chrono::Utc::now().timestamp_millis())?;
    repo.list()
}

/// 回收站列表（已软删除、保留期内的条目）
#[tauri::command]
pub fn list_deleted_collect_items(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<CollectItem>, String> {
    let repo = CollectItemRepository::new(&database);
    repo.purge_expired(chrono::Utc::now().timestamp_millis())?;
    repo.list_deleted()
}

/// 从回收站恢复
#[tauri::command]
pub fn restore_collect_item(
    database: tauri::State<'_, ApiClientDatabase>,
    item_id: String,
) -> Result<CollectItem, String> {
    CollectItemRepository::new(&database).restore(&item_id)
}

/// 彻底删除单条回收站条目
#[tauri::command]
pub fn purge_collect_item(
    database: tauri::State<'_, ApiClientDatabase>,
    item_id: String,
) -> Result<String, String> {
    CollectItemRepository::new(&database).purge(&item_id)
}

/// 清空回收站，返回清除条数
#[tauri::command]
pub fn empty_collect_trash(database: tauri::State<'_, ApiClientDatabase>) -> Result<u64, String> {
    CollectItemRepository::new(&database).purge_all_deleted()
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

//! `collect_items` 表仓库。
//!
//! 收藏夹条目的 CRUD：链接（kind=link，url 必填）与记事（kind=note，
//! content 必填）统一 upsert；记事的 url/title/description/favicon 恒为 NULL，
//! archived 跟随条目整体保存。
//!
//! 删除是软删除：打上 deleted_at 进入回收站，保留 [`RETENTION_MS`] 后
//! 由 [`Self::purge_expired`] 物理清除（命令层在每次拉列表时惰性调用）。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::collect::{normalize_kind, CollectItem, COLLECT_KIND_LINK};

/// 回收站保留期：30 天
pub const RETENTION_MS: i64 = 30 * 24 * 3600 * 1000;

pub struct CollectItemRepository<'a> {
    database: &'a ApiClientDatabase,
}

const SELECT_COLUMNS: &str = "SELECT id, kind, url, title, description, content, favicon_url,
        archived, created_at, updated_at, deleted_at
     FROM collect_items";

impl<'a> CollectItemRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    /// 未删除条目，新的在前（列表页自行按归档/类型/关键字过滤）
    pub fn list(&self) -> Result<Vec<CollectItem>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!(
                    "{SELECT_COLUMNS} WHERE deleted_at IS NULL ORDER BY created_at DESC, id DESC"
                ))
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_item)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    /// 回收站：已软删除条目，删除时间新的在前
    pub fn list_deleted(&self) -> Result<Vec<CollectItem>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!(
                    "{SELECT_COLUMNS} WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC, id DESC"
                ))
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_item)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    pub fn upsert(&self, item: &CollectItem) -> Result<CollectItem, String> {
        let kind = normalize_kind(&item.kind);
        let content = item.content.trim().to_string();

        // 链接必须有地址、记事必须有内容
        let url = if kind == COLLECT_KIND_LINK {
            Some(require_name(item.url.as_deref().unwrap_or_default(), "链接地址")?)
        } else {
            None
        };
        if kind != COLLECT_KIND_LINK && content.is_empty() {
            return Err("记事内容不能为空".to_string());
        }

        // 记事不保留链接字段，保证两种类型在库里的形态一致
        let (title, description, favicon_url) = if kind == COLLECT_KIND_LINK {
            (
                trimmed_optional(item.title.as_deref()),
                trimmed_optional(item.description.as_deref()),
                trimmed_optional(item.favicon_url.as_deref()),
            )
        } else {
            (None, None, None)
        };

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if item.id.trim().is_empty() {
                format!("collect_{}", nanoid!(10))
            } else {
                item.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO collect_items
                        (id, kind, url, title, description, content, favicon_url,
                         archived, created_at, updated_at, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
                     ON CONFLICT (id) DO UPDATE SET
                        kind = excluded.kind,
                        url = excluded.url,
                        title = excluded.title,
                        description = excluded.description,
                        content = excluded.content,
                        favicon_url = excluded.favicon_url,
                        archived = excluded.archived,
                        updated_at = excluded.updated_at,
                        deleted_at = NULL",
                    params![
                        id,
                        kind,
                        url,
                        title,
                        description,
                        content,
                        favicon_url,
                        item.archived,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            // 重新加载以返回数据库中的持久化值
            match connection.query_row(
                &format!("{SELECT_COLUMNS} WHERE id = ?1"),
                params![id],
                map_item,
            ) {
                Ok(result) => Ok(result),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    Err(format!("未找到 id 为 {id} 的收藏"))
                }
                Err(error) => Err(sql_error(error)),
            }
        })
    }

    /// 软删除：打上 deleted_at，条目进入回收站
    pub fn delete(&self, item_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let affected = connection
                .execute(
                    "UPDATE collect_items SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
                    params![item_id, now],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {item_id} 的收藏"));
            }

            Ok(item_id.to_string())
        })
    }

    /// 从回收站恢复（清除 deleted_at）
    pub fn restore(&self, item_id: &str) -> Result<CollectItem, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "UPDATE collect_items SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
                    params![item_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("回收站里没有 id 为 {item_id} 的收藏"));
            }

            connection
                .query_row(
                    &format!("{SELECT_COLUMNS} WHERE id = ?1"),
                    params![item_id],
                    map_item,
                )
                .map_err(sql_error)
        })
    }

    /// 彻底删除单条（回收站内操作）
    pub fn purge(&self, item_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM collect_items WHERE id = ?1 AND deleted_at IS NOT NULL",
                    params![item_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("回收站里没有 id 为 {item_id} 的收藏"));
            }

            Ok(item_id.to_string())
        })
    }

    /// 物理清除超过保留期的条目，返回清除数量
    pub fn purge_expired(&self, now_ms: i64) -> Result<u64, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM collect_items WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
                    params![now_ms - RETENTION_MS],
                )
                .map_err(sql_error)?;

            Ok(affected as u64)
        })
    }

    /// 清空回收站（物理删除全部已软删除条目），返回清除数量
    pub fn purge_all_deleted(&self) -> Result<u64, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM collect_items WHERE deleted_at IS NOT NULL", [])
                .map_err(sql_error)?;

            Ok(affected as u64)
        })
    }
}

fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_string)
}

fn map_item(row: &Row<'_>) -> rusqlite::Result<CollectItem> {
    let archived: i64 = row.get(7)?;

    Ok(CollectItem {
        id: row.get(0)?,
        kind: row.get(1)?,
        url: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        content: row.get(5)?,
        favicon_url: row.get(6)?,
        archived: archived != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        deleted_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_link(url: &str) -> CollectItem {
        CollectItem {
            kind: "link".to_string(),
            url: Some(url.to_string()),
            ..Default::default()
        }
    }

    fn sample_note(content: &str) -> CollectItem {
        CollectItem {
            kind: "note".to_string(),
            content: content.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn upsert_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let created = repo
            .upsert(&sample_link("https://example.com/post"))
            .expect("link should be created");
        assert!(created.id.starts_with("collect_"));
        assert_eq!(created.kind, "link");
        assert_eq!(created.url.as_deref(), Some("https://example.com/post"));
        assert!(!created.archived);

        let updated = repo
            .upsert(&CollectItem {
                title: Some("  示例文章  ".to_string()),
                description: Some("一篇说明".to_string()),
                content: "回头细读".to_string(),
                archived: true,
                ..created.clone()
            })
            .expect("link should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title.as_deref(), Some("示例文章"));
        assert_eq!(updated.description.as_deref(), Some("一篇说明"));
        assert_eq!(updated.content, "回头细读");
        assert!(updated.archived);

        // 归档状态落库后再读出来还在
        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].archived);
    }

    #[test]
    fn upsert_note_stores_content_and_clears_link_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let saved = repo
            .upsert(&sample_note("  周五前交周报  "))
            .expect("note should be created");
        assert_eq!(saved.content, "周五前交周报");
        assert_eq!(saved.url, None);
        assert_eq!(saved.title, None);

        // 即使传了链接字段，记事也会被归一化掉
        let normalized = repo
            .upsert(&CollectItem {
                url: Some("https://example.com".to_string()),
                title: Some("不该保留".to_string()),
                ..sample_note("第二条")
            })
            .expect("note with stray link fields");
        assert_eq!(normalized.url, None, "note must not keep url");
        assert_eq!(normalized.title, None, "note must not keep title");
    }

    #[test]
    fn list_orders_newest_first() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let first = repo.upsert(&sample_note("第一条")).expect("first");
        let second = repo.upsert(&sample_link("https://example.com/2")).expect("second");

        // 同毫秒创建时按 id 并列排序（随机），显式回拨第一条时间戳验证 created_at 排序
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE collect_items SET created_at = 1, updated_at = 1 WHERE id = ?1",
                        params![first.id],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .expect("backdate first item");

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
    }

    #[test]
    fn upsert_link_requires_url_and_note_requires_content() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let missing_url = CollectItem {
            kind: "link".to_string(),
            ..Default::default()
        };
        assert!(repo.upsert(&missing_url).is_err(), "link without url must be rejected");

        let blank_note = sample_note("   ");
        assert!(repo.upsert(&blank_note).is_err(), "blank note must be rejected");
    }

    #[test]
    fn upsert_normalizes_invalid_kind_to_note() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let saved = repo
            .upsert(&CollectItem {
                kind: "image".to_string(),
                content: "随手记".to_string(),
                ..Default::default()
            })
            .expect("unknown kind should not reject");
        assert_eq!(saved.kind, "note");
    }

    #[test]
    fn delete_unknown_item_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = CollectItemRepository::new(&database)
            .delete("missing")
            .expect_err("unknown item must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }

    #[test]
    fn delete_is_soft_and_restore_clears_it() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let created = repo
            .upsert(&sample_link("https://example.com/a"))
            .expect("create");
        assert_eq!(created.deleted_at, None);

        // 软删除：主列表看不到，回收站看得到且带删除时间
        repo.delete(&created.id).expect("soft delete");
        assert!(repo.list().expect("list").is_empty(), "deleted item must leave main list");

        let trash = repo.list_deleted().expect("trash");
        assert_eq!(trash.len(), 1);
        assert!(trash[0].deleted_at.is_some());
        assert_eq!(trash[0].id, created.id);

        // 重复软删除已删除的条目应报未找到（不在主列表语义里）
        assert!(repo.delete(&created.id).is_err());

        // 恢复：回到主列表，deleted_at 清空
        let restored = repo.restore(&created.id).expect("restore");
        assert_eq!(restored.deleted_at, None);
        assert_eq!(repo.list().expect("list").len(), 1);
        assert!(repo.list_deleted().expect("trash").is_empty());

        // 恢复不存在的回收站条目要报错
        assert!(repo.restore(&created.id).is_err(), "restoring an active item must fail");
    }

    #[test]
    fn purge_removes_only_trashed_items_and_purge_expired_honors_retention() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = CollectItemRepository::new(&database);

        let recent = repo.upsert(&sample_link("https://example.com/new")).expect("recent");
        let stale = repo.upsert(&sample_note("过期记事")).expect("stale");
        let active = repo.upsert(&sample_note("正常记事")).expect("active");

        repo.delete(&recent.id).expect("delete recent");
        repo.delete(&stale.id).expect("delete stale");

        // 把 stale 的删除时间拨到 31 天前，recent 保持刚删
        let cutoff_shift = RETENTION_MS + 24 * 3600 * 1000;
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE collect_items SET deleted_at = ?2 WHERE id = ?1",
                        rusqlite::params![stale.id, chrono::Utc::now().timestamp_millis() - cutoff_shift],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .expect("backdate stale deletion");

        let purged = repo
            .purge_expired(chrono::Utc::now().timestamp_millis())
            .expect("purge expired");
        assert_eq!(purged, 1, "only the stale item expires");

        let trash = repo.list_deleted().expect("trash");
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].id, recent.id, "recently deleted item must survive");
        assert_eq!(repo.list().expect("list").len(), 1, "active item must survive");

        // 彻底删除单条：只对回收站内的条目生效
        repo.purge(&recent.id).expect("purge trashed");
        assert!(repo.list_deleted().expect("trash").is_empty());
        assert!(repo.purge(&active.id).is_err(), "must not purge an active item");
        assert_eq!(repo.list().expect("list").len(), 1);
    }
}

//! `wr_repo` 与 `wr_report` 表仓库。
//!
//! 周报仓库列表与周报历史的 CRUD。仓库路径在添加时由命令层校验
//! （必须是存在的 git 仓库），仓库层只负责唯一性与落库。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::weekly_report::{WeeklyReportEntry, WeeklyReportRepo};

// ===== 周报仓库列表 =====

pub struct WeeklyReportSourceRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> WeeklyReportSourceRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<WeeklyReportRepo>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, path, name, sort, created_at, updated_at
                     FROM wr_repo ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_repo)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    /// 按 ID 批量加载，保持传入顺序；未知 ID 报错
    pub fn list_by_ids(&self, ids: &[String]) -> Result<Vec<WeeklyReportRepo>, String> {
        let all = self.list()?;

        ids.iter()
            .map(|id| {
                all.iter()
                    .find(|repo| &repo.id == id)
                    .cloned()
                    .ok_or_else(|| format!("未找到 id 为 {id} 的周报仓库"))
            })
            .collect()
    }

    /// 新增仓库。路径已存在时报错，展示名不能为空
    pub fn insert(&self, path: &str, name: &str) -> Result<WeeklyReportRepo, String> {
        let path = normalize_path(path);
        let name = require_name(name, "仓库名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let duplicated: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM wr_repo WHERE path = ?1",
                    params![path],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;
            if duplicated > 0 {
                return Err(format!("仓库已存在: {path}"));
            }

            let next_sort: i64 = transaction
                .query_row(
                    "SELECT COALESCE(MAX(sort), -1) + 1 FROM wr_repo",
                    [],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let now = Utc::now().timestamp_millis();
            let id = format!("wrre_{}", nanoid!(10));

            transaction
                .execute(
                    "INSERT INTO wr_repo (id, path, name, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, path, name, next_sort, now, now],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            Ok(WeeklyReportRepo {
                id,
                path,
                name,
                sort: next_sort,
                created_at: now,
                updated_at: now,
            })
        })
    }

    pub fn delete(&self, repo_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM wr_repo WHERE id = ?1", params![repo_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {repo_id} 的周报仓库"));
            }

            Ok(repo_id.to_string())
        })
    }
}

// ===== 周报历史 =====

pub struct WeeklyReportHistoryRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> WeeklyReportHistoryRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<WeeklyReportEntry>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, title, range_start, range_end, content, repo_count, commit_count,
                            created_at, updated_at
                     FROM wr_report ORDER BY created_at DESC, id",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_entry)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    /// 保存周报。ID 为空时创建，否则原地更新（保留创建时间）
    pub fn upsert(&self, entry: &WeeklyReportEntry) -> Result<WeeklyReportEntry, String> {
        let title = require_name(&entry.title, "周报标题")?;

        if entry.range_end < entry.range_start {
            return Err("统计区间无效：结束时间早于开始时间".to_string());
        }

        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let (id, created) = if entry.id.trim().is_empty() {
                (format!("wrpt_{}", nanoid!(10)), now)
            } else {
                let id = entry.id.trim().to_string();
                let created: i64 = connection
                    .query_row(
                        "SELECT created_at FROM wr_report WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .map_err(|error| {
                        if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                            format!("未找到 id 为 {id} 的周报")
                        } else {
                            sql_error(error)
                        }
                    })?;
                (id, created)
            };

            let transaction = begin(connection)?;

            transaction
                .execute(
                    "INSERT INTO wr_report
                        (id, title, range_start, range_end, content, repo_count, commit_count,
                         created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT (id) DO UPDATE SET
                        title = excluded.title,
                        range_start = excluded.range_start,
                        range_end = excluded.range_end,
                        content = excluded.content,
                        repo_count = excluded.repo_count,
                        commit_count = excluded.commit_count,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        title,
                        entry.range_start,
                        entry.range_end,
                        entry.content,
                        entry.repo_count,
                        entry.commit_count,
                        created,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            Ok(WeeklyReportEntry {
                id,
                title,
                range_start: entry.range_start,
                range_end: entry.range_end,
                content: entry.content.clone(),
                repo_count: entry.repo_count,
                commit_count: entry.commit_count,
                created_at: created,
                updated_at: now,
            })
        })
    }

    pub fn delete(&self, entry_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM wr_report WHERE id = ?1", params![entry_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {entry_id} 的周报"));
            }

            Ok(entry_id.to_string())
        })
    }
}

// ===== 行映射与辅助 =====

/// 统一路径分隔符并去掉尾部斜杠，作为唯一性比较的键
fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_end_matches(['/', '\\'])
        .to_string()
}

fn map_repo(row: &Row<'_>) -> rusqlite::Result<Result<WeeklyReportRepo, String>> {
    Ok(Ok(WeeklyReportRepo {
        id: row.get(0)?,
        path: row.get(1)?,
        name: row.get(2)?,
        sort: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    }))
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<Result<WeeklyReportEntry, String>> {
    Ok(Ok(WeeklyReportEntry {
        id: row.get(0)?,
        title: row.get(1)?,
        range_start: row.get(2)?,
        range_end: row.get(3)?,
        content: row.get(4)?,
        repo_count: row.get(5)?,
        commit_count: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_repo(database: &ApiClientDatabase) -> WeeklyReportSourceRepository<'_> {
        WeeklyReportSourceRepository::new(database)
    }

    fn history_repo(database: &ApiClientDatabase) -> WeeklyReportHistoryRepository<'_> {
        WeeklyReportHistoryRepository::new(database)
    }

    #[test]
    fn insert_repo_assigns_incrementing_sort_and_rejects_duplicates() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = source_repo(&database);

        let first = repo.insert("/home/user/alpha", "alpha").expect("insert");
        assert!(first.id.starts_with("wrre_"));
        assert_eq!(first.sort, 0);

        let second = repo.insert("/home/user/beta/", "beta").expect("insert");
        assert_eq!(second.sort, 1, "trailing slash is trimmed, sort increments");

        let error = repo
            .insert("/home/user/beta", "beta again")
            .expect_err("duplicate path must be rejected");
        assert!(error.contains("已存在"), "got {error}");
    }

    #[test]
    fn list_by_ids_preserves_order_and_reports_unknown_ids() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = source_repo(&database);

        let alpha = repo.insert("/repos/alpha", "alpha").expect("insert");
        let beta = repo.insert("/repos/beta", "beta").expect("insert");

        let picked = repo
            .list_by_ids(&[beta.id.clone(), alpha.id.clone()])
            .expect("list by ids");
        assert_eq!(picked.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["beta", "alpha"]);

        let error = repo
            .list_by_ids(&["missing".to_string()])
            .expect_err("unknown id must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }

    #[test]
    fn delete_unknown_repo_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = source_repo(&database)
            .delete("missing")
            .expect_err("unknown repo must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }

    #[test]
    fn upsert_entry_creates_then_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = history_repo(&database);

        let created = repo
            .upsert(&WeeklyReportEntry {
                title: "周报 9.28 - 10.4".to_string(),
                range_start: 1,
                range_end: 2,
                content: "# 周报".to_string(),
                repo_count: 2,
                commit_count: 17,
                ..Default::default()
            })
            .expect("create");
        assert!(created.id.starts_with("wrpt_"));

        let updated = repo
            .upsert(&WeeklyReportEntry {
                id: created.id.clone(),
                title: "周报（修订）".to_string(),
                content: "# 周报 v2".to_string(),
                ..created.clone()
            })
            .expect("update");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "周报（修订）");
        assert_eq!(updated.created_at, created.created_at);

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].content, "# 周报 v2");
    }

    #[test]
    fn upsert_entry_validates_title_and_range() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = history_repo(&database);

        let error = repo
            .upsert(&WeeklyReportEntry {
                title: " ".to_string(),
                ..Default::default()
            })
            .expect_err("empty title must be rejected");
        assert!(error.contains("不能为空"), "got {error}");

        let error = repo
            .upsert(&WeeklyReportEntry {
                title: "周报".to_string(),
                range_start: 10,
                range_end: 5,
                ..Default::default()
            })
            .expect_err("inverted range must be rejected");
        assert!(error.contains("区间无效"), "got {error}");
    }

    #[test]
    fn update_missing_entry_is_rejected_and_delete_reports_unknown() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = history_repo(&database);

        let error = repo
            .upsert(&WeeklyReportEntry {
                id: "missing".to_string(),
                title: "周报".to_string(),
                ..Default::default()
            })
            .expect_err("updating unknown entry must be rejected");
        assert!(error.contains("未找到"), "got {error}");

        let error = repo
            .delete("missing")
            .expect_err("deleting unknown entry must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

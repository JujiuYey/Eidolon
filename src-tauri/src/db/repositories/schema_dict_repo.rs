//! `schema_databases` / `schema_snapshots` 表仓库。
//!
//! 数据字典的库条目与快照 CRUD。快照随库级联删除（FK ON DELETE CASCADE）。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::schema_dict::{
    normalize_db_type, normalize_source_kind, SchemaDatabase, SchemaSnapshot,
};

const DATABASE_COLUMNS: &str =
    "SELECT id, name, db_type, note, sort, created_at, updated_at FROM schema_databases";

const SNAPSHOT_COLUMNS: &str = "SELECT id, database_id, label, source_kind, source_text,
        tables_json, note, created_at, updated_at
     FROM schema_snapshots";

// ===== 库条目 =====

pub struct SchemaDatabaseRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> SchemaDatabaseRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<SchemaDatabase>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!("{DATABASE_COLUMNS} ORDER BY sort, created_at, id"))
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_database)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    pub fn upsert(&self, item: &SchemaDatabase) -> Result<SchemaDatabase, String> {
        let name = require_name(&item.name, "库名称")?;
        let db_type = normalize_db_type(&item.db_type);

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if item.id.trim().is_empty() {
                format!("sdb_{}", nanoid!(10))
            } else {
                item.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO schema_databases
                        (id, name, db_type, note, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        db_type = excluded.db_type,
                        note = excluded.note,
                        sort = excluded.sort,
                        updated_at = excluded.updated_at",
                    params![id, name, db_type, item.note.trim(), item.sort, now, now],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            let saved = match connection.query_row(
                &format!("{DATABASE_COLUMNS} WHERE id = ?1"),
                params![id],
                map_database,
            ) {
                Ok(result) => Ok(result),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的库条目"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            saved
        })
    }

    pub fn delete(&self, id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM schema_databases WHERE id = ?1", params![id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {id} 的库条目"));
            }

            Ok(id.to_string())
        })
    }
}

// ===== 快照 =====

pub struct SchemaSnapshotRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> SchemaSnapshotRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    /// 某库的快照列表，按创建时间升序（时间线顺序）
    pub fn list_by_database(&self, database_id: &str) -> Result<Vec<SchemaSnapshot>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!(
                    "{SNAPSHOT_COLUMNS} WHERE database_id = ?1 ORDER BY created_at, id"
                ))
                .map_err(sql_error)?;

            let rows = statement
                .query_map(params![database_id], map_snapshot)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    pub fn get(&self, snapshot_id: &str) -> Result<Option<SchemaSnapshot>, String> {
        self.database.with_connection(|connection| {
            let result = connection
                .query_row(
                    &format!("{SNAPSHOT_COLUMNS} WHERE id = ?1"),
                    params![snapshot_id],
                    map_snapshot,
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => String::new(),
                    other => sql_error(other),
                });

            match result {
                Ok(inner) => Ok(Some(inner)),
                Err(message) if message.is_empty() => Ok(None),
                Err(message) => Err(message),
            }
        })
    }

    pub fn upsert(&self, item: &SchemaSnapshot) -> Result<SchemaSnapshot, String> {
        let label = require_name(&item.label, "快照版本名")?;
        let database_id = require_name(&item.database_id, "所属库")?;
        let source_kind = normalize_source_kind(&item.source_kind);

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            // 新建时若前端带回原 created_at（>0）则保留，避免重排序
            let created_at = if item.created_at > 0 { item.created_at } else { now };
            let id = if item.id.trim().is_empty() {
                format!("ssn_{}", nanoid!(10))
            } else {
                item.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO schema_snapshots
                        (id, database_id, label, source_kind, source_text, tables_json, note,
                         created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT (id) DO UPDATE SET
                        label = excluded.label,
                        source_kind = excluded.source_kind,
                        source_text = excluded.source_text,
                        tables_json = excluded.tables_json,
                        note = excluded.note,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        database_id,
                        label,
                        source_kind,
                        item.source_text,
                        item.tables_json,
                        item.note.trim(),
                        created_at,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            let saved = match connection.query_row(
                &format!("{SNAPSHOT_COLUMNS} WHERE id = ?1"),
                params![id],
                map_snapshot,
            ) {
                Ok(result) => Ok(result),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的快照"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            saved
        })
    }

    pub fn delete(&self, snapshot_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM schema_snapshots WHERE id = ?1", params![snapshot_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {snapshot_id} 的快照"));
            }

            Ok(snapshot_id.to_string())
        })
    }
}

// ===== 行映射 =====

fn map_database(row: &Row<'_>) -> rusqlite::Result<SchemaDatabase> {
    Ok(SchemaDatabase {
        id: row.get(0)?,
        name: row.get(1)?,
        db_type: row.get(2)?,
        note: row.get(3)?,
        sort: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_snapshot(row: &Row<'_>) -> rusqlite::Result<SchemaSnapshot> {
    Ok(SchemaSnapshot {
        id: row.get(0)?,
        database_id: row.get(1)?,
        label: row.get(2)?,
        source_kind: row.get(3)?,
        source_text: row.get(4)?,
        tables_json: row.get(5)?,
        note: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_database(name: &str) -> SchemaDatabase {
        SchemaDatabase {
            name: name.to_string(),
            db_type: "pg".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn database_crud_round_trip() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SchemaDatabaseRepository::new(&database);

        let created = repo
            .upsert(&sample_database("病案质控库"))
            .expect("created");
        assert!(created.id.starts_with("sdb_"));
        assert_eq!(created.db_type, "pg");

        let updated = repo
            .upsert(&SchemaDatabase {
                name: "体检库".to_string(),
                db_type: "oracle ".to_string(),
                ..created.clone()
            })
            .expect("updated");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.db_type, "oracle", "db_type should be normalized");

        assert_eq!(repo.list().expect("list").len(), 1);

        repo.delete(&created.id).expect("deleted");
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn snapshots_cascade_with_database_and_order_by_created_at() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let db_repo = SchemaDatabaseRepository::new(&database);
        let snap_repo = SchemaSnapshotRepository::new(&database);

        let library = db_repo.upsert(&sample_database("体检库")).expect("created");

        for (label, created_at) in [("第二版", 200), ("第一版", 100)] {
            snap_repo
                .upsert(&SchemaSnapshot {
                    database_id: library.id.clone(),
                    label: label.to_string(),
                    source_kind: "ddl".to_string(),
                    source_text: "--".to_string(),
                    tables_json: r#"{"tables":[]}"#.to_string(),
                    created_at,
                    ..Default::default()
                })
                .expect("snapshot created");
        }

        let listed = snap_repo.list_by_database(&library.id).expect("listed");
        let labels: Vec<&str> = listed.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["第一版", "第二版"], "ordered by created_at");

        let fetched = snap_repo.get(&listed[0].id).expect("get").expect("exists");
        assert_eq!(fetched.source_kind, "ddl");

        // 删库级联删快照
        db_repo.delete(&library.id).expect("database deleted");
        let remaining = snap_repo.list_by_database(&library.id).expect("listed");
        assert!(remaining.is_empty(), "snapshots should cascade");
    }

    #[test]
    fn upsert_rejects_blank_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let db_repo = SchemaDatabaseRepository::new(&database);
        let snap_repo = SchemaSnapshotRepository::new(&database);

        assert!(db_repo.upsert(&sample_database("  ")).is_err());

        let blank_label = SchemaSnapshot {
            label: "  ".to_string(),
            ..Default::default()
        };
        assert!(snap_repo.upsert(&blank_label).is_err());
    }

    #[test]
    fn delete_unknown_returns_error() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        assert!(SchemaDatabaseRepository::new(&database).delete("missing").is_err());
        assert!(SchemaSnapshotRepository::new(&database).delete("missing").is_err());
    }
}

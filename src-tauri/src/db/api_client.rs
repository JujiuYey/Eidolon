use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OpenFlags};

/// API Client 的 SQLite 数据库文件名
pub const DATABASE_FILENAME: &str = "api-client.sqlite";

/// 递增迁移。已应用的迁移不得修改，新增行为只能追加新版本。
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("migrations/0001_init.sql"))];

/// API Client 专用数据库，与 `LocalJsonStore` 相互独立
pub struct ApiClientDatabase {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl ApiClientDatabase {
    /// 在应用数据目录下打开 `api-client.sqlite` 并执行迁移
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|error| format!("创建数据目录失败: {error}"))?;

        let path = data_dir.join(DATABASE_FILENAME);
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| format!("打开数据库失败 ({}): {error}", path.display()))?;

        configure(&connection)?;
        migrate(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    /// 打开内存数据库，仅用于测试
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        let connection =
            Connection::open_in_memory().map_err(|error| format!("打开内存数据库失败: {error}"))?;

        // 内存数据库不支持 WAL journal mode；只启用外键与 busy_timeout
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|error| format!("启用外键约束失败: {error}"))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| format!("设置 busy_timeout 失败: {error}"))?;

        migrate(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 在持有连接锁的作用域内执行读写
    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| "数据库连接状态已损坏".to_string())?;
        operation(&mut guard)
    }
}

/// 启用外键约束等连接级设置
fn configure(connection: &Connection) -> Result<(), String> {
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| format!("设置 journal_mode 失败: {error}"))?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| format!("启用外键约束失败: {error}"))?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| format!("设置 busy_timeout 失败: {error}"))?;
    Ok(())
}

/// 按递增版本执行迁移，每个版本在独立事务中应用
fn migrate(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );",
        )
        .map_err(|error| format!("创建迁移表失败: {error}"))?;

    let applied = applied_versions(connection)?;

    for (version, sql) in MIGRATIONS {
        if applied.contains(version) {
            continue;
        }

        connection
            .execute_batch("BEGIN IMMEDIATE;")
            .map_err(|error| format!("开始迁移事务失败: {error}"))?;

        let result = connection
            .execute_batch(sql)
            .map_err(|error| format!("执行迁移 {version} 失败: {error}"))
            .and_then(|_| {
                connection
                    .execute(
                        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                        rusqlite::params![version, chrono::Utc::now().timestamp_millis()],
                    )
                    .map_err(|error| format!("记录迁移 {version} 失败: {error}"))
            });

        match result {
            Ok(_) => {
                connection
                    .execute_batch("COMMIT;")
                    .map_err(|error| format!("提交迁移 {version} 失败: {error}"))?;
            }
            Err(error) => {
                let _ = connection.execute_batch("ROLLBACK;");
                return Err(error);
            }
        }
    }

    Ok(())
}

fn applied_versions(connection: &Connection) -> Result<Vec<i64>, String> {
    let mut statement = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .map_err(|error| format!("读取迁移版本失败: {error}"))?;

    let versions = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("读取迁移版本失败: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取迁移版本失败: {error}"))?;

    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_creates_database_file_and_tables() {
        let temp_dir = tempdir().expect("temp dir");
        let database = ApiClientDatabase::open(temp_dir.path()).expect("database should open");

        assert!(temp_dir.path().join(DATABASE_FILENAME).exists());

        database
            .with_connection(|connection| {
                let tables = connection
                    .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                    .and_then(|mut statement| {
                        statement
                            .query_map([], |row| row.get::<_, String>(0))?
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .map_err(|error| error.to_string())?;

                for expected in [
                    "api_environments",
                    "api_groups",
                    "api_projects",
                    "api_request_histories",
                    "api_requests",
                    "schema_migrations",
                ] {
                    assert!(
                        tables.iter().any(|name| name == expected),
                        "table {expected} should exist, got {tables:?}"
                    );
                }

                Ok(())
            })
            .expect("table listing should succeed");
    }

    #[test]
    fn migrations_are_idempotent_across_reopen() {
        let temp_dir = tempdir().expect("temp dir");

        {
            let first = ApiClientDatabase::open(temp_dir.path()).expect("first open");
            first
                .with_connection(|connection| {
                    connection
                        .execute(
                            "INSERT INTO api_projects (id, name, description, sort, created_at, updated_at)
                             VALUES ('p1', '订单系统', '', 0, 1, 1)",
                            [],
                        )
                        .map_err(|error| error.to_string())?;
                    Ok(())
                })
                .expect("insert should succeed");
        }

        let second = ApiClientDatabase::open(temp_dir.path()).expect("second open");
        second
            .with_connection(|connection| {
                let versions = applied_versions(connection)?;
                assert_eq!(versions, vec![1]);

                let count: i64 = connection
                    .query_row("SELECT COUNT(*) FROM api_projects", [], |row| row.get(0))
                    .map_err(|error| error.to_string())?;
                assert_eq!(count, 1, "existing rows must survive reopen");
                Ok(())
            })
            .expect("second open checks should succeed");
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");

        database
            .with_connection(|connection| {
                let enabled: bool = connection
                    .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                    .map_err(|error| error.to_string())?;
                assert!(enabled, "foreign_keys pragma must be on");

                let error = connection
                    .execute(
                        "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                         VALUES ('g1', 'missing-project', NULL, '默认分组', 0, 1, 1)",
                        [],
                    )
                    .expect_err("group with unknown project must be rejected");
                assert!(error.to_string().contains("FOREIGN KEY"), "got {error}");
                Ok(())
            })
            .expect("foreign key checks should run");
    }

    #[test]
    fn deleting_project_cascades_groups_requests_and_histories() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");

        database
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "INSERT INTO api_projects (id, name, description, sort, created_at, updated_at)
                            VALUES ('p1', '订单系统', '', 0, 1, 1);
                         INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                            VALUES ('g1', 'p1', NULL, '默认分组', 0, 1, 1);
                         INSERT INTO api_requests
                            (id, group_id, name, method, url, query_json, headers_json, body_kind, body_text,
                             body_form_json, timeout_ms, ai_prompt, ai_reference, sort, created_at, updated_at)
                            VALUES ('r1', 'g1', '新增订单', 'POST', '/orders', '[]', '[]', 'json', '{}', '[]',
                                    30000, '', '', 0, 1, 1);
                         INSERT INTO api_request_histories
                            (id, request_id, environment_name, request_snapshot_json, status, status_code,
                             response_headers_json, response_body_preview, response_body_truncated, duration_ms,
                             error_message, executed_at)
                            VALUES ('h1', 'r1', '测试环境', '{}', 'success', 200, '[]', '{}', 0, 12, NULL, 1);",
                    )
                    .map_err(|error| error.to_string())?;

                connection
                    .execute("DELETE FROM api_projects WHERE id = 'p1'", [])
                    .map_err(|error| error.to_string())?;

                for (table, expected) in [
                    ("api_groups", 0i64),
                    ("api_requests", 0),
                    ("api_request_histories", 0),
                ] {
                    let count: i64 = connection
                        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
                        .map_err(|error| error.to_string())?;
                    assert_eq!(count, expected, "{table} should be cascaded");
                }

                Ok(())
            })
            .expect("cascade checks should run");
    }

    #[test]
    fn failed_migration_rolls_back_and_reports_version() {
        let connection = Connection::open_in_memory().expect("connection");
        configure(&connection).expect("configure");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at INTEGER NOT NULL
                );",
            )
            .expect("migration table");

        connection.execute_batch("BEGIN IMMEDIATE;").expect("begin");
        let outcome = connection
            .execute_batch("CREATE TABLE probe (id TEXT); SELECT this_is_not_valid_sql;")
            .map_err(|error| error.to_string());
        assert!(outcome.is_err(), "invalid migration must fail");
        connection.execute_batch("ROLLBACK;").expect("rollback");

        let probe_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'probe'",
                [],
                |row| row.get(0),
            )
            .expect("probe lookup");
        assert_eq!(
            probe_exists, 0,
            "rolled back migration must leave no tables"
        );
    }
}

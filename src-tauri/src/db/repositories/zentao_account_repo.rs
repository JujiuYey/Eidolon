//! `zentao_accounts` 表仓库。
//!
//! 禅道账户的 CRUD。密码遵循项目"掩码而非加密"约定，明文存储在本地 SQLite；
//! 任务/Bug 数据不落库，每次实时拉取。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::zentao::ZentaoAccount;

pub struct ZentaoAccountRepository<'a> {
    database: &'a ApiClientDatabase,
}

const SELECT_COLUMNS: &str = "SELECT id, name, base_url, account, password,
        enabled, sort, created_at, updated_at
     FROM zentao_accounts";

impl<'a> ZentaoAccountRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<ZentaoAccount>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!("{SELECT_COLUMNS} ORDER BY sort, created_at, id"))
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_account)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    pub fn get(&self, account_id: &str) -> Result<Option<ZentaoAccount>, String> {
        self.database.with_connection(|connection| {
            let result = connection
                .query_row(
                    &format!("{SELECT_COLUMNS} WHERE id = ?1"),
                    params![account_id],
                    map_account,
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

    pub fn upsert(&self, account: &ZentaoAccount) -> Result<ZentaoAccount, String> {
        let name = require_name(&account.name, "账户名称")?;
        let base_url = require_name(&account.base_url, "禅道站点地址")?;
        let login_account = require_name(&account.account, "登录账号")?;
        let password = account.password.trim().to_string();

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if account.id.trim().is_empty() {
                format!("zta_{}", nanoid!(10))
            } else {
                account.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO zentao_accounts
                        (id, name, base_url, account, password, enabled, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        base_url = excluded.base_url,
                        account = excluded.account,
                        password = excluded.password,
                        enabled = excluded.enabled,
                        sort = excluded.sort,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        name,
                        base_url,
                        login_account,
                        password,
                        account.enabled as i64,
                        account.sort,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            // 重新加载以返回数据库中的持久化值
            let saved = match connection
                .query_row(
                    &format!("{SELECT_COLUMNS} WHERE id = ?1"),
                    params![id],
                    map_account,
                ) {
                Ok(result) => Ok(result),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的禅道账户"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            saved
        })
    }

    pub fn delete(&self, account_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM zentao_accounts WHERE id = ?1",
                    params![account_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {account_id} 的禅道账户"));
            }

            Ok(account_id.to_string())
        })
    }
}

fn map_account(row: &Row<'_>) -> rusqlite::Result<ZentaoAccount> {
    let enabled: i64 = row.get(5)?;

    Ok(ZentaoAccount {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        account: row.get(3)?,
        password: row.get(4)?,
        enabled: enabled != 0,
        sort: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(name: &str) -> ZentaoAccount {
        ZentaoAccount {
            name: name.to_string(),
            base_url: "http://192.168.10.209".to_string(),
            account: "hjc".to_string(),
            password: "secret".to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn upsert_account_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = ZentaoAccountRepository::new(&database);

        let created = repo
            .upsert(&sample_account("公司禅道"))
            .expect("account should be created");
        assert!(created.id.starts_with("zta_"));
        assert!(created.enabled);

        let updated = repo
            .upsert(&ZentaoAccount {
                name: "测试环境禅道".to_string(),
                base_url: "http://192.168.10.210/zentao".to_string(),
                ..created.clone()
            })
            .expect("account should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "测试环境禅道");
        assert_eq!(updated.base_url, "http://192.168.10.210/zentao");

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);

        let deleted = repo.delete(&created.id).expect("delete should succeed");
        assert_eq!(deleted, created.id);
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn upsert_account_keeps_password_round_trip() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = ZentaoAccountRepository::new(&database);

        let created = repo
            .upsert(&sample_account("公司禅道"))
            .expect("account should be created");

        let loaded = repo
            .get(&created.id)
            .expect("get")
            .expect("account exists");
        assert_eq!(loaded.password, "secret");
        assert_eq!(loaded.account, "hjc");
    }

    #[test]
    fn upsert_account_rejects_invalid_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = ZentaoAccountRepository::new(&database);

        let missing_name = ZentaoAccount {
            name: "  ".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_name).is_err());

        let missing_base_url = ZentaoAccount {
            base_url: String::new(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_base_url).is_err());

        let missing_login = ZentaoAccount {
            account: "  ".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_login).is_err());
    }

    #[test]
    fn delete_unknown_account_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = ZentaoAccountRepository::new(&database)
            .delete("missing")
            .expect_err("unknown account must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

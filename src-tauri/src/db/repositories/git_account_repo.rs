//! `git_accounts` 表仓库。
//!
//! Git 账户的 CRUD。每个账户代表用户的一个 git 提交身份（显示名 + 邮箱），
//! 周报「只看我的提交」按启用账号的邮箱匹配提交作者。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::email::validate_email;
use crate::models::git::GitAccount;

pub struct GitAccountRepository<'a> {
    database: &'a ApiClientDatabase,
}

const SELECT_COLUMNS: &str = "SELECT id, name, email,
        enabled, sort, created_at, updated_at
     FROM git_accounts";

impl<'a> GitAccountRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<GitAccount>, String> {
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

    pub fn upsert(&self, account: &GitAccount) -> Result<GitAccount, String> {
        let name = require_name(&account.name, "账户名称")?;
        let email = validate_email(&account.email, "Git 邮箱")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if account.id.trim().is_empty() {
                format!("gacc_{}", nanoid!(10))
            } else {
                account.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO git_accounts
                        (id, name, email, enabled, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        email = excluded.email,
                        enabled = excluded.enabled,
                        sort = excluded.sort,
                        updated_at = excluded.updated_at",
                    params![id, name, email, account.enabled as i64, account.sort, now, now],
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
                    return Err(format!("未找到 id 为 {id} 的 Git 账户"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            saved
        })
    }

    pub fn delete(&self, account_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM git_accounts WHERE id = ?1", params![account_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {account_id} 的 Git 账户"));
            }

            Ok(account_id.to_string())
        })
    }
}

fn map_account(row: &Row<'_>) -> rusqlite::Result<GitAccount> {
    let enabled: i64 = row.get(3)?;

    Ok(GitAccount {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        enabled: enabled != 0,
        sort: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(name: &str) -> GitAccount {
        GitAccount {
            name: name.to_string(),
            email: "me@company.com".to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn upsert_account_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = GitAccountRepository::new(&database);

        let created = repo
            .upsert(&sample_account("公司电脑"))
            .expect("account should be created");
        assert!(created.id.starts_with("gacc_"));
        assert!(created.enabled);

        let updated = repo
            .upsert(&GitAccount {
                name: "家里笔记本".to_string(),
                email: "me@personal.com".to_string(),
                ..created.clone()
            })
            .expect("account should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "家里笔记本");
        assert_eq!(updated.email, "me@personal.com");

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);

        let deleted = repo.delete(&created.id).expect("delete should succeed");
        assert_eq!(deleted, created.id);
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn upsert_account_trims_email() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = GitAccountRepository::new(&database);

        let created = repo
            .upsert(&GitAccount {
                email: "  me@company.com  ".to_string(),
                ..sample_account("公司电脑")
            })
            .expect("account should be created");
        assert_eq!(created.email, "me@company.com");
    }

    #[test]
    fn upsert_account_rejects_invalid_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = GitAccountRepository::new(&database);

        let missing_name = GitAccount {
            name: "  ".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_name).is_err());

        let missing_email = GitAccount {
            email: String::new(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_email).is_err());

        let bad_email = GitAccount {
            email: "not-an-email".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&bad_email).is_err());
    }

    #[test]
    fn delete_unknown_account_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = GitAccountRepository::new(&database)
            .delete("missing")
            .expect_err("unknown account must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

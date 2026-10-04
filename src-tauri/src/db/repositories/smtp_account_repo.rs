//! `smtp_accounts` 表仓库。
//!
//! 发信账户的 CRUD。`email` 同时作为 SMTP 登录用户名与发件地址；
//! 密码遵循项目"掩码而非加密"约定，明文存储在本地 SQLite。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::email::{validate_email, validate_port, SmtpAccount, SmtpEncryption};

pub struct SmtpAccountRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> SmtpAccountRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<SmtpAccount>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, name, email, password, host, port, encryption, from_name,
                            enabled, sort, created_at, updated_at
                     FROM smtp_accounts ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_account)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get(&self, account_id: &str) -> Result<Option<SmtpAccount>, String> {
        self.database.with_connection(|connection| {
            let result = connection
                .query_row(
                    "SELECT id, name, email, password, host, port, encryption, from_name,
                            enabled, sort, created_at, updated_at
                     FROM smtp_accounts WHERE id = ?1",
                    params![account_id],
                    map_account,
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => String::new(),
                    other => sql_error(other),
                });

            match result {
                Ok(inner) => inner.map(Some),
                Err(message) if message.is_empty() => Ok(None),
                Err(message) => Err(message),
            }
        })
    }

    pub fn upsert(&self, account: &SmtpAccount) -> Result<SmtpAccount, String> {
        let name = require_name(&account.name, "账户名称")?;
        let email = validate_email(&account.email, "邮箱地址")?;
        let host = require_name(&account.host, "SMTP 服务器地址")?;
        let port = validate_port(account.port)?;
        let password = account.password.trim().to_string();
        let from_name = account.from_name.trim().to_string();

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if account.id.trim().is_empty() {
                format!("sacc_{}", nanoid!(10))
            } else {
                account.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO smtp_accounts
                        (id, name, email, password, host, port, encryption, from_name,
                         enabled, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        email = excluded.email,
                        password = excluded.password,
                        host = excluded.host,
                        port = excluded.port,
                        encryption = excluded.encryption,
                        from_name = excluded.from_name,
                        enabled = excluded.enabled,
                        sort = excluded.sort,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        name,
                        email,
                        password,
                        host,
                        port,
                        account.encryption.as_str(),
                        from_name,
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
                    "SELECT id, name, email, password, host, port, encryption, from_name,
                            enabled, sort, created_at, updated_at
                     FROM smtp_accounts WHERE id = ?1",
                    params![id],
                    map_account,
                ) {
                Ok(result) => result?,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的邮件账户"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            Ok(saved)
        })
    }

    pub fn delete(&self, account_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM smtp_accounts WHERE id = ?1",
                    params![account_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {account_id} 的邮件账户"));
            }

            Ok(account_id.to_string())
        })
    }
}

fn map_account(row: &Row<'_>) -> rusqlite::Result<Result<SmtpAccount, String>> {
    let encryption_text: String = row.get(6)?;
    let enabled: i64 = row.get(8)?;

    let account = SmtpAccount {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        password: row.get(3)?,
        host: row.get(4)?,
        port: row.get(5)?,
        encryption: SmtpEncryption::Tls,
        from_name: row.get(7)?,
        enabled: enabled != 0,
        sort: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    };

    Ok((|| {
        Ok(SmtpAccount {
            encryption: SmtpEncryption::parse(&encryption_text)?,
            ..account
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(name: &str) -> SmtpAccount {
        SmtpAccount {
            name: name.to_string(),
            email: "user@example.com".to_string(),
            password: "auth-code".to_string(),
            host: "smtp.example.com".to_string(),
            port: 465,
            encryption: SmtpEncryption::Tls,
            from_name: "张三".to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn upsert_account_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SmtpAccountRepository::new(&database);

        let created = repo
            .upsert(&sample_account("公司邮箱"))
            .expect("account should be created");
        assert!(created.id.starts_with("sacc_"));
        assert_eq!(created.encryption, SmtpEncryption::Tls);

        let updated = repo
            .upsert(&SmtpAccount {
                name: "个人邮箱".to_string(),
                port: 587,
                encryption: SmtpEncryption::Starttls,
                ..created.clone()
            })
            .expect("account should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "个人邮箱");
        assert_eq!(updated.port, 587);
        assert_eq!(updated.encryption, SmtpEncryption::Starttls);

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);

        let deleted = repo.delete(&created.id).expect("delete should succeed");
        assert_eq!(deleted, created.id);
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn upsert_account_keeps_password_round_trip() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SmtpAccountRepository::new(&database);

        let created = repo
            .upsert(&sample_account("公司邮箱"))
            .expect("account should be created");

        let loaded = repo
            .get(&created.id)
            .expect("get")
            .expect("account exists");
        assert_eq!(loaded.password, "auth-code");
    }

    #[test]
    fn upsert_account_rejects_invalid_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SmtpAccountRepository::new(&database);

        let missing_name = SmtpAccount {
            name: "  ".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&missing_name).is_err());

        let bad_email = SmtpAccount {
            email: "not-an-email".to_string(),
            ..sample_account("x")
        };
        assert!(repo.upsert(&bad_email).is_err());

        let bad_port = SmtpAccount {
            port: 0,
            ..sample_account("x")
        };
        assert!(repo.upsert(&bad_port).is_err());
    }

    #[test]
    fn delete_unknown_account_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = SmtpAccountRepository::new(&database)
            .delete("missing")
            .expect_err("unknown account must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

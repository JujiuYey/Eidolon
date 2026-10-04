//! `sent_emails` 表仓库。
//!
//! 发送历史的写入与查询：每次发送（含失败）追加一条，
//! 正文受 1 MiB 截断，全局只保留最近 `MAX_SENT_EMAIL_HISTORY` 条。
//! 账户/模板删除时通过外键 `ON DELETE SET NULL` 保留历史。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{sql_error, begin};
use crate::models::email::{
    EmailSendStatus, SentEmail, MAX_EMAIL_BODY_BYTES, MAX_SENT_EMAIL_HISTORY,
};

pub struct SentEmailRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> SentEmailRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<SentEmail>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, account_id, account_email, to_addresses, cc_addresses,
                            subject, body, template_id, status, error_message, sent_at, created_at
                     FROM sent_emails ORDER BY sent_at DESC, id DESC",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_sent_email)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    /// 追加一条发送历史。`account_id`/`template_id` 为空时不建立外键关联
    pub fn append(&self, record: &SentEmail) -> Result<SentEmail, String> {
        let mut record = record.clone();

        if record.id.trim().is_empty() {
            record.id = format!("sent_{}", nanoid!(10));
        }

        if record.sent_at == 0 {
            record.sent_at = Utc::now().timestamp_millis();
        }

        let (body, truncated) = truncate_body(&record.body);
        record.body = body;
        let _ = truncated;

        let created_at = if record.created_at == 0 {
            Utc::now().timestamp_millis()
        } else {
            record.created_at
        };
        record.created_at = created_at;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            transaction
                .execute(
                    "INSERT INTO sent_emails
                        (id, account_id, account_email, to_addresses, cc_addresses,
                         subject, body, template_id, status, error_message, sent_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        record.id,
                        record.account_id,
                        record.account_email,
                        record.to_addresses,
                        record.cc_addresses,
                        record.subject,
                        record.body,
                        record.template_id,
                        record.status.as_str(),
                        record.error_message,
                        record.sent_at,
                        record.created_at
                    ],
                )
                .map_err(sql_error)?;

            // 全局只保留最近 MAX_SENT_EMAIL_HISTORY 条
            transaction
                .execute(
                    "DELETE FROM sent_emails WHERE id NOT IN (
                        SELECT id FROM sent_emails
                        ORDER BY sent_at DESC, id DESC
                        LIMIT ?1
                     )",
                    params![MAX_SENT_EMAIL_HISTORY as i64],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(record)
        })
    }

    pub fn delete(&self, email_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM sent_emails WHERE id = ?1", params![email_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {email_id} 的发送记录"));
            }

            Ok(email_id.to_string())
        })
    }

    pub fn clear(&self) -> Result<usize, String> {
        self.database.with_connection(|connection| {
            connection
                .execute("DELETE FROM sent_emails", [])
                .map_err(sql_error)
        })
    }
}

/// 历史正文上限截断，按字符边界切分避免破坏 UTF-8
fn truncate_body(body: &str) -> (String, bool) {
    if body.len() <= MAX_EMAIL_BODY_BYTES {
        return (body.to_string(), false);
    }

    let mut end = MAX_EMAIL_BODY_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }

    (body[..end].to_string(), true)
}

fn map_sent_email(row: &Row<'_>) -> rusqlite::Result<Result<SentEmail, String>> {
    let status_text: String = row.get(8)?;

    let record = SentEmail {
        id: row.get(0)?,
        account_id: row.get(1)?,
        account_email: row.get(2)?,
        to_addresses: row.get(3)?,
        cc_addresses: row.get(4)?,
        subject: row.get(5)?,
        body: row.get(6)?,
        template_id: row.get(7)?,
        status: EmailSendStatus::Sent,
        error_message: row.get(9)?,
        sent_at: row.get(10)?,
        created_at: row.get(11)?,
    };

    Ok((|| {
        Ok(SentEmail {
            status: EmailSendStatus::parse(&status_text)?,
            ..record
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(status: EmailSendStatus) -> SentEmail {
        SentEmail {
            account_email: "user@example.com".to_string(),
            to_addresses: "boss@example.com".to_string(),
            subject: "张三的请假申请".to_string(),
            body: "尊敬的领导：".to_string(),
            status,
            ..Default::default()
        }
    }

    #[test]
    fn append_stores_and_lists_records_newest_first() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SentEmailRepository::new(&database);

        for index in 0..3 {
            repo.append(&SentEmail {
                subject: format!("第{index}封"),
                sent_at: 1_000 + index,
                ..sample_record(EmailSendStatus::Sent)
            })
            .expect("record should be written");
        }

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].subject, "第2封", "newest must come first");
        assert!(listed[0].id.starts_with("sent_"));
        assert_eq!(listed[0].status, EmailSendStatus::Sent);
    }

    #[test]
    fn append_keeps_only_recent_entries() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SentEmailRepository::new(&database);

        for index in 0..(MAX_SENT_EMAIL_HISTORY + 5) {
            repo.append(&SentEmail {
                subject: format!("第{index}封"),
                sent_at: 1_000 + index as i64,
                ..sample_record(EmailSendStatus::Sent)
            })
            .expect("record should be written");
        }

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), MAX_SENT_EMAIL_HISTORY);
        assert!(
            !listed.iter().any(|record| record.subject == "第0封"),
            "oldest entries must be pruned"
        );
    }

    #[test]
    fn append_truncates_body_beyond_limit() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SentEmailRepository::new(&database);

        let oversized = "a".repeat(MAX_EMAIL_BODY_BYTES + 1024);
        let saved = repo
            .append(&SentEmail {
                body: oversized,
                ..sample_record(EmailSendStatus::Sent)
            })
            .expect("record should be written");

        assert_eq!(saved.body.len(), MAX_EMAIL_BODY_BYTES);
    }

    #[test]
    fn deleting_account_keeps_history_with_account_id_cleared() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");

        let account = crate::db::repositories::smtp_account_repo::SmtpAccountRepository::new(
            &database,
        )
        .upsert(&crate::models::email::SmtpAccount {
            name: "公司邮箱".to_string(),
            email: "user@example.com".to_string(),
            host: "smtp.example.com".to_string(),
            port: 465,
            ..Default::default()
        })
        .expect("account should be created");

        let repo = SentEmailRepository::new(&database);
        repo.append(&SentEmail {
            account_id: Some(account.id.clone()),
            ..sample_record(EmailSendStatus::Sent)
        })
        .expect("record should be written");

        crate::db::repositories::smtp_account_repo::SmtpAccountRepository::new(&database)
            .delete(&account.id)
            .expect("account should be deleted");

        let listed = repo.list().expect("history survives account deletion");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].account_id, None);
        assert_eq!(listed[0].account_email, "user@example.com");
    }

    #[test]
    fn delete_and_clear_remove_records() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SentEmailRepository::new(&database);

        let first = repo
            .append(&sample_record(EmailSendStatus::Sent))
            .expect("record should be written");
        repo.append(&sample_record(EmailSendStatus::Failed))
            .expect("record should be written");

        repo.delete(&first.id).expect("delete should succeed");
        assert_eq!(repo.list().expect("list").len(), 1);

        assert_eq!(repo.clear().expect("clear should succeed"), 1);
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn failed_status_round_trips_through_database() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = SentEmailRepository::new(&database);

        repo.append(&SentEmail {
            error_message: Some("连接超时".to_string()),
            ..sample_record(EmailSendStatus::Failed)
        })
        .expect("record should be written");

        let listed = repo.list().expect("list");
        assert_eq!(listed[0].status, EmailSendStatus::Failed);
        assert_eq!(listed[0].error_message.as_deref(), Some("连接超时"));
    }
}

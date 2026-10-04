//! `mail_templates` 表仓库。
//!
//! 邮件模板的 CRUD。主题/正文保存含 `{{变量}}` 的原文，渲染在前端完成；
//! 内置模板由迁移 `0002_email.sql` 预置，删除后不会恢复。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::email::MailTemplate;

pub struct MailTemplateRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> MailTemplateRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<MailTemplate>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, name, subject, body, sort, created_at, updated_at
                     FROM mail_templates ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_template)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            rows.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn upsert(&self, template: &MailTemplate) -> Result<MailTemplate, String> {
        let name = require_name(&template.name, "模板名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if template.id.trim().is_empty() {
                format!("mtpl_{}", nanoid!(10))
            } else {
                template.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO mail_templates
                        (id, name, subject, body, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        subject = excluded.subject,
                        body = excluded.body,
                        sort = excluded.sort,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        name,
                        template.subject,
                        template.body,
                        template.sort,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;

            let saved = match connection
                .query_row(
                    "SELECT id, name, subject, body, sort, created_at, updated_at
                     FROM mail_templates WHERE id = ?1",
                    params![id],
                    map_template,
                ) {
                Ok(result) => result?,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的邮件模板"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            Ok(saved)
        })
    }

    pub fn delete(&self, template_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM mail_templates WHERE id = ?1",
                    params![template_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {template_id} 的邮件模板"));
            }

            Ok(template_id.to_string())
        })
    }
}

fn map_template(row: &Row<'_>) -> rusqlite::Result<Result<MailTemplate, String>> {
    Ok(Ok(MailTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        subject: row.get(2)?,
        body: row.get(3)?,
        sort: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_seeds_builtin_templates_once() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = MailTemplateRepository::new(&database);

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 3, "migration should seed three builtin templates");
        assert!(listed
            .iter()
            .any(|template| template.id == "mtpl_builtin_leave"));
        assert!(listed
            .iter()
            .all(|template| template.subject.contains("{{")));
    }

    #[test]
    fn upsert_template_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = MailTemplateRepository::new(&database);

        let created = repo
            .upsert(&MailTemplate {
                name: "周报".to_string(),
                subject: "{{姓名}}的周报（{{日期}}）".to_string(),
                body: "本周完成：{{本周工作}}".to_string(),
                ..Default::default()
            })
            .expect("template should be created");
        assert!(created.id.starts_with("mtpl_"));

        let updated = repo
            .upsert(&MailTemplate {
                name: "周报（修订）".to_string(),
                ..created.clone()
            })
            .expect("template should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "周报（修订）");

        let deleted = repo.delete(&created.id).expect("delete should succeed");
        assert_eq!(deleted, created.id);
    }

    #[test]
    fn upsert_template_rejects_empty_name() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = MailTemplateRepository::new(&database)
            .upsert(&MailTemplate {
                name: " ".to_string(),
                ..Default::default()
            })
            .expect_err("empty name must be rejected");
        assert!(error.contains("不能为空"), "got {error}");
    }

    #[test]
    fn delete_unknown_template_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = MailTemplateRepository::new(&database)
            .delete("missing")
            .expect_err("unknown template must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

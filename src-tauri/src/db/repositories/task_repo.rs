//! `tasks` 表仓库。
//!
//! 个人任务的 CRUD。status 变为 done 时补记完成时间，改回未完成时清除。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{begin, require_name, sql_error};
use crate::models::task::{
    normalize_due_date, normalize_priority, normalize_status, Task, TASK_STATUS_DONE,
};

pub struct TaskRepository<'a> {
    database: &'a ApiClientDatabase,
}

const SELECT_COLUMNS: &str = "SELECT id, title, notes, status, priority, due_date, source,
        sort, completed_at, created_at, updated_at
     FROM tasks";

impl<'a> TaskRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<Task>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(&format!("{SELECT_COLUMNS} ORDER BY sort, created_at, id"))
                .map_err(sql_error)?;

            let rows = statement
                .query_map([], map_task)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(rows)
        })
    }

    pub fn upsert(&self, task: &Task) -> Result<Task, String> {
        let title = require_name(&task.title, "任务标题")?;
        let status = normalize_status(&task.status);
        let priority = normalize_priority(&task.priority);
        let due_date = normalize_due_date(&task.due_date);
        let source = if task.source.trim().is_empty() {
            "manual".to_string()
        } else {
            task.source.trim().to_string()
        };

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let now = Utc::now().timestamp_millis();
            let id = if task.id.trim().is_empty() {
                format!("task_{}", nanoid!(10))
            } else {
                task.id.trim().to_string()
            };

            // 完成时间状态机：变为 done 时补记，改回未完成时清除
            let completed_at = if status == TASK_STATUS_DONE {
                Some(task.completed_at.unwrap_or(now))
            } else {
                None
            };

            transaction
                .execute(
                    "INSERT INTO tasks
                        (id, title, notes, status, priority, due_date, source, sort,
                         completed_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT (id) DO UPDATE SET
                        title = excluded.title,
                        notes = excluded.notes,
                        status = excluded.status,
                        priority = excluded.priority,
                        due_date = excluded.due_date,
                        source = excluded.source,
                        sort = excluded.sort,
                        completed_at = excluded.completed_at,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        title,
                        task.notes.trim(),
                        status,
                        priority,
                        due_date,
                        source,
                        task.sort,
                        completed_at,
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
                    map_task,
                ) {
                Ok(result) => Ok(result),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("未找到 id 为 {id} 的任务"));
                }
                Err(error) => return Err(sql_error(error)),
            };

            saved
        })
    }

    pub fn delete(&self, task_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute("DELETE FROM tasks WHERE id = ?1", params![task_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {task_id} 的任务"));
            }

            Ok(task_id.to_string())
        })
    }
}

fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let due_date: Option<String> = row.get(5)?;
    let completed_at: Option<i64> = row.get(8)?;

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        due_date: due_date.unwrap_or_default(),
        source: row.get(6)?,
        sort: row.get(7)?,
        completed_at,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(title: &str) -> Task {
        Task {
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn upsert_task_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = TaskRepository::new(&database);

        let created = repo
            .upsert(&sample_task("写周报"))
            .expect("task should be created");
        assert!(created.id.starts_with("task_"));
        assert_eq!(created.status, "todo");
        assert_eq!(created.priority, "none");
        assert_eq!(created.completed_at, None);

        let updated = repo
            .upsert(&Task {
                title: "整理周报数据".to_string(),
                notes: "先拉 git 提交记录".to_string(),
                status: "doing".to_string(),
                priority: "high".to_string(),
                due_date: "2026-10-15".to_string(),
                ..created.clone()
            })
            .expect("task should update in place");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "整理周报数据");
        assert_eq!(updated.status, "doing");
        assert_eq!(updated.priority, "high");
        assert_eq!(updated.due_date, "2026-10-15");
        assert_eq!(updated.notes, "先拉 git 提交记录");
        assert_eq!(updated.completed_at, None);

        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);

        let deleted = repo.delete(&created.id).expect("delete should succeed");
        assert_eq!(deleted, created.id);
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn completing_task_stamps_completed_at_and_reverting_clears_it() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = TaskRepository::new(&database);

        let task = repo.upsert(&sample_task("回复评审")).expect("create");

        let done = repo
            .upsert(&Task {
                status: "done".to_string(),
                completed_at: None,
                ..task.clone()
            })
            .expect("complete");
        assert!(done.completed_at.is_some(), "done task must be stamped");

        // 保持 done 再次保存，完成时间不重置
        let still_done = repo
            .upsert(&Task {
                title: "回复评审意见".to_string(),
                status: "done".to_string(),
                completed_at: done.completed_at,
                ..done.clone()
            })
            .expect("update while done");
        assert_eq!(still_done.completed_at, done.completed_at);

        let reopened = repo
            .upsert(&Task {
                status: "todo".to_string(),
                ..still_done.clone()
            })
            .expect("reopen");
        assert_eq!(reopened.completed_at, None);
    }

    #[test]
    fn upsert_task_normalizes_invalid_fields() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = TaskRepository::new(&database);

        let saved = repo
            .upsert(&Task {
                title: "  验证归一化  ".to_string(),
                status: "archived".to_string(),
                priority: "urgent".to_string(),
                due_date: "周五".to_string(),
                ..sample_task("x")
            })
            .expect("invalid enum values should not reject");

        assert_eq!(saved.title, "验证归一化");
        assert_eq!(saved.status, "todo", "unknown status falls back to todo");
        assert_eq!(saved.priority, "none", "unknown priority falls back to none");
        assert_eq!(saved.due_date, "", "unparseable due date is dropped");
        assert_eq!(saved.source, "manual");
    }

    #[test]
    fn upsert_task_rejects_blank_title() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let repo = TaskRepository::new(&database);

        let blank = Task {
            title: "   ".to_string(),
            ..sample_task("x")
        };
        assert!(repo.upsert(&blank).is_err());
    }

    #[test]
    fn delete_unknown_task_is_rejected() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = TaskRepository::new(&database)
            .delete("missing")
            .expect_err("unknown task must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

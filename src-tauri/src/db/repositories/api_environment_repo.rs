//! `api_environments` 表仓库。
//!
//! 环境按项目持有，保存请求体生成或执行时所用的 `base_url` 与变量键值行。
//! 项目归属校验由仓库入口把关，跨项目使用环境时由 `api_request_repo`
//! 中的 `resolve_environment` 进一步比对。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, from_json, require_name, require_project, sql_error, to_json,
};
use crate::models::api_client::ApiEnvironment;

pub struct ApiEnvironmentRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiEnvironmentRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self, project_id: &str) -> Result<Vec<ApiEnvironment>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, project_id, name, base_url, variables_json, created_at, updated_at
                     FROM api_environments WHERE project_id = ?1 ORDER BY name, id",
                )
                .map_err(sql_error)?;

            let environments = statement
                .query_map(params![project_id], map_environment)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            environments.into_iter().collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn upsert(&self, environment: &ApiEnvironment) -> Result<ApiEnvironment, String> {
        let name = require_name(&environment.name, "环境名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, &environment.project_id)?;

            let now = Utc::now().timestamp_millis();
            let id = if environment.id.trim().is_empty() {
                format!("aenv_{}", nanoid!(10))
            } else {
                environment.id.trim().to_string()
            };

            transaction
                .execute(
                    "INSERT INTO api_environments
                        (id, project_id, name, base_url, variables_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (id) DO UPDATE SET
                        name = excluded.name,
                        base_url = excluded.base_url,
                        variables_json = excluded.variables_json,
                        updated_at = excluded.updated_at",
                    params![
                        id,
                        environment.project_id,
                        name,
                        environment.base_url.trim(),
                        to_json(&environment.variables)?,
                        now,
                        now
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            crate::db::repositories::api_client_repo_common::load_environment(connection, &id)
        })
    }

    pub fn delete(&self, environment_id: &str) -> Result<String, String> {
        self.database.with_connection(|connection| {
            let affected = connection
                .execute(
                    "DELETE FROM api_environments WHERE id = ?1",
                    params![environment_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {environment_id} 的环境"));
            }

            Ok(environment_id.to_string())
        })
    }
}

pub(crate) fn map_environment(row: &Row<'_>) -> rusqlite::Result<Result<ApiEnvironment, String>> {
    let variables_json: String = row.get(4)?;
    let environment = ApiEnvironment {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        base_url: row.get(3)?,
        variables: Vec::new(),
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    };

    Ok((|| {
        Ok(ApiEnvironment {
            variables: from_json(&variables_json)?,
            ..environment
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::api_project_repo::ApiProjectRepository;
    use crate::models::api_client::KeyValueRow;

    #[test]
    fn upsert_environment_creates_and_updates_in_place() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let project = ApiProjectRepository::new(&database)
            .create("订单系统", "")
            .expect("project");
        let repo = ApiEnvironmentRepository::new(&database);

        let environment = repo
            .upsert(&ApiEnvironment {
                project_id: project.id.clone(),
                name: "测试环境".to_string(),
                base_url: "https://test.example.com".to_string(),
                variables: vec![KeyValueRow::new("token", "abc")],
                ..Default::default()
            })
            .expect("environment should be created");

        assert_eq!(environment.variables.len(), 1);

        let updated = repo
            .upsert(&ApiEnvironment {
                name: "预发环境".to_string(),
                ..environment.clone()
            })
            .expect("environment should update in place");
        assert_eq!(updated.id, environment.id);
        assert_eq!(updated.name, "预发环境");

        let listed = repo.list(&project.id).expect("list");
        assert_eq!(listed.len(), 1);

        let deleted = repo.delete(&environment.id).expect("delete should succeed");
        assert_eq!(deleted, environment.id);
        assert!(repo.list(&project.id).expect("list").is_empty());
    }

    #[test]
    fn upsert_environment_rejects_unknown_project() {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let error = ApiEnvironmentRepository::new(&database)
            .upsert(&ApiEnvironment {
                project_id: "missing".to_string(),
                name: "测试环境".to_string(),
                ..Default::default()
            })
            .expect_err("unknown project must be rejected");
        assert!(error.contains("未找到"), "got {error}");
    }
}

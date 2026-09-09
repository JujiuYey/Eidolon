//! `api_groups` 表仓库。
//!
//! 分组位于项目之下，首版只有一层；删除分组通过外键级联清除其下
//! 请求与历史，关联数量由 `delete` 返回供 UI 展示。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, next_sort, require_name, require_project, sql_error, DeletionSummary,
};
use crate::models::api_client::ApiGroup;

pub struct ApiGroupRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiGroupRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    pub fn list(&self, project_id: &str) -> Result<Vec<ApiGroup>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, project_id, parent_group_id, name, sort, created_at, updated_at
                     FROM api_groups WHERE project_id = ?1 ORDER BY sort, created_at, id",
                )
                .map_err(sql_error)?;

            let groups = statement
                .query_map(params![project_id], map_group)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(groups)
        })
    }

    pub fn create(&self, project_id: &str, name: &str) -> Result<ApiGroup, String> {
        let name = require_name(name, "分组名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_groups WHERE project_id = ?1",
                params![project_id],
            )?;

            let group = ApiGroup {
                id: format!("agr_{}", nanoid!(10)),
                project_id: project_id.to_string(),
                parent_group_id: None,
                name,
                sort,
                created_at: now,
                updated_at: now,
            };

            transaction
                .execute(
                    "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
                    params![
                        group.id,
                        group.project_id,
                        group.name,
                        group.sort,
                        group.created_at,
                        group.updated_at
                    ],
                )
                .map_err(sql_error)?;

            transaction.commit().map_err(sql_error)?;
            Ok(group)
        })
    }

    pub fn rename(&self, group_id: &str, name: &str) -> Result<ApiGroup, String> {
        let name = require_name(name, "分组名称")?;

        self.database.with_connection(|connection| {
            let now = Utc::now().timestamp_millis();
            let affected = connection
                .execute(
                    "UPDATE api_groups SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    params![name, now, group_id],
                )
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {group_id} 的分组"));
            }

            load_group(connection, group_id)
        })
    }

    pub fn delete(&self, group_id: &str) -> Result<DeletionSummary, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let deleted_requests: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM api_requests WHERE group_id = ?1",
                    params![group_id],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let deleted_histories: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM api_request_histories h
                     JOIN api_requests r ON r.id = h.request_id
                     WHERE r.group_id = ?1",
                    params![group_id],
                    |row| row.get(0),
                )
                .map_err(sql_error)?;

            let affected = transaction
                .execute("DELETE FROM api_groups WHERE id = ?1", params![group_id])
                .map_err(sql_error)?;

            if affected == 0 {
                return Err(format!("未找到 id 为 {group_id} 的分组"));
            }

            transaction.commit().map_err(sql_error)?;

            Ok(DeletionSummary {
                deleted_groups: 1,
                deleted_requests,
                deleted_histories,
            })
        })
    }

    /// 按给定顺序重排分组，顺序稳定且只影响该项目
    pub fn reorder(&self, project_id: &str, group_ids: &[String]) -> Result<Vec<ApiGroup>, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            let now = Utc::now().timestamp_millis();

            for (index, group_id) in group_ids.iter().enumerate() {
                let affected = transaction
                    .execute(
                        "UPDATE api_groups SET sort = ?1, updated_at = ?2
                         WHERE id = ?3 AND project_id = ?4",
                        params![index as i64, now, group_id, project_id],
                    )
                    .map_err(sql_error)?;

                if affected == 0 {
                    return Err(format!("分组 {group_id} 不属于项目 {project_id}"));
                }
            }

            transaction.commit().map_err(sql_error)?;
            Ok(())
        })?;

        self.list(project_id)
    }
}

fn map_group(row: &Row<'_>) -> rusqlite::Result<ApiGroup> {
    Ok(ApiGroup {
        id: row.get(0)?,
        project_id: row.get(1)?,
        parent_group_id: row.get(2)?,
        name: row.get(3)?,
        sort: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn load_group(connection: &Connection, group_id: &str) -> Result<ApiGroup, String> {
    connection
        .query_row(
            "SELECT id, project_id, parent_group_id, name, sort, created_at, updated_at
             FROM api_groups WHERE id = ?1",
            params![group_id],
            map_group,
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| format!("未找到 id 为 {group_id} 的分组"))
}

#[cfg(test)]
mod tests {
    use crate::db::api_client::ApiClientDatabase;
    use crate::db::repositories::api_group_repo::ApiGroupRepository;
    use crate::db::repositories::api_project_repo::ApiProjectRepository;

    fn seeded() -> (ApiClientDatabase, String) {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let project_id = ApiProjectRepository::new(&database)
            .create("订单系统", "")
            .expect("project")
            .id;
        (database, project_id)
    }

    #[test]
    fn create_group_then_rename_and_delete_reports_counts() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let group = repo
            .create(&project_id, "订单查询")
            .expect("group should be created");
        assert_eq!(group.project_id, project_id);
        assert_eq!(group.parent_group_id, None);

        let renamed = repo
            .rename(&group.id, "订单详情")
            .expect("rename should succeed");
        assert_eq!(renamed.name, "订单详情");

        let deleted = repo.delete(&group.id).expect("delete should succeed");
        assert_eq!(deleted.deleted_groups, 1);
        assert_eq!(deleted.deleted_requests, 0);
    }

    #[test]
    fn reorder_groups_is_stable_and_scoped_to_project() {
        let (database, project_id) = seeded();
        let group_repo = ApiGroupRepository::new(&database);
        let initial = group_repo.list(&project_id).expect("groups");
        let default_group = initial[0].clone();
        let second = group_repo.create(&project_id, "订单查询").expect("group b");
        let third = group_repo.create(&project_id, "订单统计").expect("group c");

        let reordered = group_repo
            .reorder(
                &project_id,
                &[
                    third.id.clone(),
                    default_group.id.clone(),
                    second.id.clone(),
                ],
            )
            .expect("reorder should succeed");

        let ids = reordered
            .iter()
            .map(|group| group.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                third.id.as_str(),
                default_group.id.as_str(),
                second.id.as_str()
            ]
        );

        let other_project = ApiProjectRepository::new(&database)
            .create("库存系统", "")
            .expect("other project");
        let error = group_repo
            .reorder(&other_project.id, &[second.id.clone()])
            .expect_err("group from another project must be rejected");
        assert!(error.contains("不属于项目"), "got {error}");
    }
}

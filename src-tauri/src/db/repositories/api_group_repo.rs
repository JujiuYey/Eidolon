//! `api_groups` 表仓库。
//!
//! 分组位于项目之下，支持 N 级嵌套；删除分组通过外键级联清除其下
//! 请求与历史，以及所有后代分组；关联数量由 `delete` 返回供 UI 展示。

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, group_project_id, next_sort, require_name, require_project, sql_error,
    DeletionSummary,
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

    pub fn create(
        &self,
        project_id: &str,
        parent_group_id: Option<&str>,
        name: &str,
    ) -> Result<ApiGroup, String> {
        let name = require_name(name, "分组名称")?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;

            if let Some(parent_id) = parent_group_id {
                let parent_project = group_project_id(&transaction, parent_id)?;
                if parent_project != project_id {
                    return Err(format!(
                        "父分组 {parent_id} 不属于项目 {project_id}"
                    ));
                }
            }

            let now = Utc::now().timestamp_millis();
            let sort = next_sort(
                &transaction,
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_groups WHERE project_id = ?1",
                params![project_id],
            )?;

            let group = ApiGroup {
                id: format!("agr_{}", nanoid!(10)),
                project_id: project_id.to_string(),
                parent_group_id: parent_group_id.map(|value| value.to_string()),
                name,
                sort,
                created_at: now,
                updated_at: now,
            };

            transaction
                .execute(
                    "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        group.id,
                        group.project_id,
                        group.parent_group_id,
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

    /// 调整父分组：`Some(parent_id)` 表示移入该父分组之下，`None` 表示提升为顶级。
    /// 拒绝自指、跨项目与成环；移动后保持原 `sort`，仅顶级重排会被 `reorder` 影响。
    pub fn move_to(
        &self,
        group_id: &str,
        parent_group_id: Option<&str>,
    ) -> Result<ApiGroup, String> {
        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;

            let current = load_group(&transaction, group_id)?;

            let new_parent = parent_group_id.map(|value| value.to_string());

            if let Some(parent_id) = new_parent.as_deref() {
                if parent_id == group_id {
                    return Err("分组不能将自身设为父分组".to_string());
                }

                let parent_project = group_project_id(&transaction, parent_id)?;
                if parent_project != current.project_id {
                    return Err(format!(
                        "父分组 {parent_id} 不属于项目 {}",
                        current.project_id
                    ));
                }

                // 沿父链向上走，若任何祖先等于 group_id 即成环。
                let mut cursor: Option<String> = Some(parent_id.to_string());
                while let Some(ancestor_id) = cursor.as_deref() {
                    if ancestor_id == group_id {
                        return Err(format!(
                            "将分组 {group_id} 移入父分组 {parent_id} 会产生循环"
                        ));
                    }
                    let next: Option<String> = transaction
                        .query_row(
                            "SELECT parent_group_id FROM api_groups WHERE id = ?1",
                            params![ancestor_id],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(sql_error)?
                        .flatten();
                    cursor = next;
                }
            }

            let now = Utc::now().timestamp_millis();
            let affected = transaction
                .execute(
                    "UPDATE api_groups SET parent_group_id = ?1, updated_at = ?2 WHERE id = ?3",
                    params![new_parent, now, group_id],
                )
                .map_err(sql_error)?;
            if affected == 0 {
                return Err(format!("未找到 id 为 {group_id} 的分组"));
            }

            transaction.commit().map_err(sql_error)?;
            load_group(connection, group_id)
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
            .create(&project_id, None, "订单查询")
            .expect("group should be created");
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
        let second = group_repo.create(&project_id, None, "订单查询").expect("group b");
        let third = group_repo.create(&project_id, None, "订单统计").expect("group c");
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

    #[test]
    fn create_child_group_attaches_parent() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let parent = repo
            .create(&project_id, None, "订单")
            .expect("parent should be created");
        let child = repo
            .create(&project_id, Some(&parent.id), "订单查询")
            .expect("child should be created");

        assert_eq!(child.parent_group_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(child.project_id, project_id);

        // 在项目下按 sort 列出仍能看到父子两行。
        let listed = repo.list(&project_id).expect("list groups");
        let listed_ids: Vec<&str> =
            listed.iter().map(|group| group.id.as_str()).collect();
        assert!(listed_ids.contains(&parent.id.as_str()));
        assert!(listed_ids.contains(&child.id.as_str()));
    }

    #[test]
    fn move_group_to_child_then_back_to_root() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let parent = repo
            .create(&project_id, None, "订单")
            .expect("parent");
        let child = repo
            .create(&project_id, None, "订单查询")
            .expect("child");

        let moved = repo
            .move_to(&child.id, Some(&parent.id))
            .expect("move into parent should succeed");
        assert_eq!(moved.parent_group_id.as_deref(), Some(parent.id.as_str()));

        let promoted = repo
            .move_to(&child.id, None)
            .expect("move to root should succeed");
        assert_eq!(promoted.parent_group_id, None);
    }

    #[test]
    fn move_group_rejects_self_parent() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let group = repo
            .create(&project_id, None, "订单")
            .expect("group");

        let error = repo
            .move_to(&group.id, Some(&group.id))
            .expect_err("self-parent must be rejected");
        assert!(error.contains("不能将自身设为父分组"), "got {error}");
    }

    #[test]
    fn move_group_rejects_cycle() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let root = repo.create(&project_id, None, "root").expect("root");
        let middle = repo
            .create(&project_id, Some(&root.id), "middle")
            .expect("middle");
        let leaf = repo
            .create(&project_id, Some(&middle.id), "leaf")
            .expect("leaf");

        // 试图把 root 移入 leaf，应被识别为成环（leaf -> middle -> root）。
        let error = repo
            .move_to(&root.id, Some(&leaf.id))
            .expect_err("cycle must be rejected");
        assert!(error.contains("循环"), "got {error}");

        // 链路结构应保持不变。
        let root_after = repo.list(&project_id).expect("list").into_iter()
            .find(|group| group.id == root.id)
            .expect("root still present");
        assert_eq!(root_after.parent_group_id, None);
    }

    #[test]
    fn move_group_rejects_cross_project_parent() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let other_project = ApiProjectRepository::new(&database)
            .create("库存系统", "")
            .expect("other project");
        let other_default = repo.list(&other_project.id).expect("list other")[0]
            .id
 .clone();

        let group = repo
            .create(&project_id, None, "订单")
            .expect("group");

        let error = repo
            .move_to(&group.id, Some(&other_default))
            .expect_err("cross-project parent must be rejected");
        assert!(error.contains("不属于项目"), "got {error}");
    }

    #[test]
    fn delete_group_cascades_to_descendants() {
        let (database, project_id) = seeded();
        let repo = ApiGroupRepository::new(&database);

        let parent = repo
            .create(&project_id, None, "订单")
            .expect("parent");
        let child = repo
            .create(&project_id, Some(&parent.id), "订单查询")
            .expect("child");
        let grand = repo
            .create(&project_id, Some(&child.id), "订单详情")
            .expect("grand");

        let deleted = repo.delete(&parent.id).expect("delete should succeed");
        assert_eq!(deleted.deleted_groups, 1);

        let remaining = repo.list(&project_id).expect("list");
        let remaining_ids: Vec<&str> =
            remaining.iter().map(|group| group.id.as_str()).collect();
        assert!(!remaining_ids.contains(&parent.id.as_str()));
        assert!(!remaining_ids.contains(&child.id.as_str()));
        assert!(!remaining_ids.contains(&grand.id.as_str()));
    }
}

//! 代码导入仓库：保存「项目 ↔ 后端仓库」导入源，并把前端适配器
//! 解析出的分组/请求清单在一个事务里落库。
//!
//! 导入为重建语义：`replace_existing` 时先清空项目下的旧分组
//! （级联删除其请求与历史），再按清单建立「模块 > 资源」两级分组。
//! 每个请求可携带 `external_key`（来源接口标识），手工请求不受影响。

use std::collections::HashMap;

use chrono::Utc;
use nanoid::nanoid;
use rusqlite::{params, OptionalExtension};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_client_repo_common::{
    begin, require_name, require_project, sql_error,
};
use crate::models::api_client::{
    normalize_method, ApiImportSource, ImportGroupSpec, ImportRequestSpec, ImportSummary,
};

pub struct ApiImportRepository<'a> {
    database: &'a ApiClientDatabase,
}

impl<'a> ApiImportRepository<'a> {
    pub fn new(database: &'a ApiClientDatabase) -> Self {
        Self { database }
    }

    /// 保存导入源。每个项目最多一条：已存在则更新框架与路径。
    pub fn save_source(
        &self,
        project_id: &str,
        framework: &str,
        repo_path: &str,
    ) -> Result<ApiImportSource, String> {
        let framework = framework.trim();
        if framework.is_empty() {
            return Err("框架类型不能为空".to_string());
        }
        let repo_path = repo_path.trim();
        if repo_path.is_empty() {
            return Err("仓库路径不能为空".to_string());
        }

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;

            let now = Utc::now().timestamp_millis();
            let existing: Option<String> = transaction
                .query_row(
                    "SELECT id FROM api_import_sources WHERE project_id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_error)?;

            let source = match existing {
                Some(id) => {
                    transaction
                        .execute(
                            "UPDATE api_import_sources
                             SET framework = ?1, repo_path = ?2, updated_at = ?3
                             WHERE id = ?4",
                            params![framework, repo_path, now, id],
                        )
                        .map_err(sql_error)?;
                    ApiImportSource {
                        id,
                        project_id: project_id.to_string(),
                        framework: framework.to_string(),
                        repo_path: repo_path.to_string(),
                        created_at: now,
                        updated_at: now,
                    }
                }
                None => {
                    let source = ApiImportSource {
                        id: format!("aimp_{}", nanoid!(10)),
                        project_id: project_id.to_string(),
                        framework: framework.to_string(),
                        repo_path: repo_path.to_string(),
                        created_at: now,
                        updated_at: now,
                    };
                    transaction
                        .execute(
                            "INSERT INTO api_import_sources (id, project_id, framework, repo_path, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![
                                source.id,
                                source.project_id,
                                source.framework,
                                source.repo_path,
                                source.created_at,
                                source.updated_at
                            ],
                        )
                        .map_err(sql_error)?;
                    source
                }
            };

            transaction.commit().map_err(sql_error)?;
            Ok(source)
        })
    }

    pub fn list_sources(&self, project_id: &str) -> Result<Vec<ApiImportSource>, String> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, project_id, framework, repo_path, created_at, updated_at
                     FROM api_import_sources WHERE project_id = ?1",
                )
                .map_err(sql_error)?;

            let sources = statement
                .query_map(params![project_id], |row| {
                    Ok(ApiImportSource {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        framework: row.get(2)?,
                        repo_path: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                })
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?;

            Ok(sources)
        })
    }

    /// 执行导入：校验清单后在单事务里重建分组并批量插入请求。
    pub fn run_import(
        &self,
        project_id: &str,
        replace_existing: bool,
        groups: &[ImportGroupSpec],
        requests: &[ImportRequestSpec],
    ) -> Result<ImportSummary, String> {
        validate_specs(groups, requests)?;

        self.database.with_connection(|connection| {
            let transaction = begin(connection)?;
            require_project(&transaction, project_id)?;

            let mut summary = ImportSummary::default();

            if replace_existing {
                summary.deleted_groups = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM api_groups WHERE project_id = ?1",
                        params![project_id],
                        |row| row.get(0),
                    )
                    .map_err(sql_error)?;
                summary.deleted_requests = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM api_requests r
                         JOIN api_groups g ON g.id = r.group_id
                         WHERE g.project_id = ?1",
                        params![project_id],
                        |row| row.get(0),
                    )
                    .map_err(sql_error)?;
                summary.deleted_histories = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM api_request_histories h
                         JOIN api_requests r ON r.id = h.request_id
                         JOIN api_groups g ON g.id = r.group_id
                         WHERE g.project_id = ?1",
                        params![project_id],
                        |row| row.get(0),
                    )
                    .map_err(sql_error)?;

                // 分组的父外键与请求外键均级联删除，删顶级行即清空全部
                transaction
                    .execute(
                        "DELETE FROM api_groups WHERE project_id = ?1",
                        params![project_id],
                    )
                    .map_err(sql_error)?;
            }

            let now = Utc::now().timestamp_millis();
            let mut group_ids: HashMap<&str, String> = HashMap::new();

            // 按依赖顺序插入分组：父键已解析（或为顶级）的先插，
            // 循环直到全部落位；校验阶段已排除环与未知父键。
            let mut pending: Vec<&ImportGroupSpec> = groups.iter().collect();
            while !pending.is_empty() {
                let mut remaining: Vec<&ImportGroupSpec> = Vec::new();
                let mut progressed = false;

                for spec in pending {
                    let parent_resolved = match spec.parent_key.as_deref() {
                        None => true,
                        Some(parent_key) => group_ids.contains_key(parent_key),
                    };
                    if !parent_resolved {
                        remaining.push(spec);
                        continue;
                    }

                    let parent_id = spec
                        .parent_key
                        .as_deref()
                        .and_then(|parent_key| group_ids.get(parent_key))
                        .cloned();

                    let sort = next_group_sort(&transaction, project_id, parent_id.as_deref())?;

                    let id = format!("agr_{}", nanoid!(10));
                    transaction
                        .execute(
                            "INSERT INTO api_groups (id, project_id, parent_group_id, name, sort, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                            params![id, project_id, parent_id, spec.name, sort, now],
                        )
                        .map_err(sql_error)?;
                    summary.created_groups += 1;
                    group_ids.insert(spec.key.as_str(), id);
                    progressed = true;
                }

                if !progressed {
                    return Err("分组之间存在无法解析的父子关系".to_string());
                }
                pending = remaining;
            }

            let mut request_sorts: HashMap<String, i64> = HashMap::new();
            for spec in requests {
                let group_id = group_ids
                    .get(spec.group_key.as_str())
                    .ok_or_else(|| format!("请求 {} 引用了不存在的分组键", spec.name))?;

                let name = require_name(&spec.name, "请求名称")?;
                let method = normalize_method(&spec.method)?;
                if spec.timeout_ms == 0 {
                    return Err(format!("请求 {name} 的超时时间必须大于 0"));
                }

                let sort = match request_sorts.get(group_id) {
                    Some(current) => *current,
                    None => transaction
                        .query_row(
                            "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_requests WHERE group_id = ?1",
                            params![group_id],
                            |row| row.get(0),
                        )
                        .map_err(sql_error)?,
                };
                request_sorts.insert(group_id.clone(), sort + 1);

                let headers_json = serde_json::to_string(&spec.headers)
                    .map_err(|error| format!("序列化请求头失败: {error}"))?;
                let query_json = serde_json::to_string(&spec.query)
                    .map_err(|error| format!("序列化查询参数失败: {error}"))?;
                let form_json = serde_json::to_string(&spec.body.form)
                    .map_err(|error| format!("序列化表单失败: {error}"))?;

                transaction
                    .execute(
                        "INSERT INTO api_requests
                            (id, group_id, name, method, url, query_json, headers_json,
                             body_kind, body_text, body_form_json, timeout_ms, ai_prompt,
                             ai_reference, external_key, sort, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, '', '', ?12, ?13, ?14, ?14)",
                        params![
                            format!("areq_{}", nanoid!(10)),
                            group_id,
                            name,
                            method,
                            spec.url.trim(),
                            query_json,
                            headers_json,
                            spec.body.kind.as_str(),
                            spec.body.text,
                            form_json,
                            spec.timeout_ms as i64,
                            spec.external_key,
                            sort,
                            now
                        ],
                    )
                    .map_err(sql_error)?;
                summary.created_requests += 1;
            }

            transaction.commit().map_err(sql_error)?;
            Ok(summary)
        })
    }
}

/// 事务前做纯校验：分组键唯一、父键可解析（含环检测）、
/// 请求引用的分组键存在。
fn validate_specs(
    groups: &[ImportGroupSpec],
    requests: &[ImportRequestSpec],
) -> Result<(), String> {
    let mut keys: HashMap<&str, ()> = HashMap::new();
    for spec in groups {
        require_name(&spec.name, "分组名称")?;
        let key = spec.key.trim();
        if key.is_empty() {
            return Err("分组键不能为空".to_string());
        }
        if keys.insert(key, ()).is_some() {
            return Err(format!("分组键重复: {key}"));
        }
    }

    for spec in groups {
        if let Some(parent_key) = spec.parent_key.as_deref() {
            if parent_key == spec.key {
                return Err(format!("分组 {} 不能以自身为父分组", spec.key));
            }
            if !keys.contains_key(parent_key) {
                return Err(format!(
                    "分组 {} 引用了不存在的父分组键 {}",
                    spec.key, parent_key
                ));
            }
        }
    }

    for spec in requests {
        if !keys.contains_key(spec.group_key.as_str()) {
            return Err(format!(
                "请求 {} 引用了不存在的分组键 {}",
                spec.name, spec.group_key
            ));
        }
    }

    Ok(())
}

/// 计算分组在指定父级下的下一个排序位，`None` 表示顶级
fn next_group_sort(
    transaction: &rusqlite::Transaction<'_>,
    project_id: &str,
    parent_id: Option<&str>,
) -> Result<i64, String> {
    match parent_id {
        None => transaction
            .query_row(
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_groups
                 WHERE project_id = ?1 AND parent_group_id IS NULL",
                params![project_id],
                |row| row.get(0),
            )
            .map_err(sql_error),
        Some(parent) => transaction
            .query_row(
                "SELECT COALESCE(MAX(sort), -1) + 1 FROM api_groups
                 WHERE project_id = ?1 AND parent_group_id = ?2",
                params![project_id, parent],
                |row| row.get(0),
            )
            .map_err(sql_error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::api_group_repo::ApiGroupRepository;
    use crate::db::repositories::api_project_repo::ApiProjectRepository;
    use crate::db::repositories::api_request_repo::ApiRequestRepository;
    use crate::models::api_client::{
        BodyKind, ImportGroupSpec, ImportRequestSpec, KeyValueRow, RequestBody,
    };

    fn seeded() -> (ApiClientDatabase, String) {
        let database = ApiClientDatabase::open_in_memory().expect("in-memory database");
        let project_id = ApiProjectRepository::new(&database)
            .create("智慧管理", "")
            .expect("project")
            .id;
        (database, project_id)
    }

    fn module_spec() -> Vec<ImportGroupSpec> {
        vec![
            ImportGroupSpec {
                key: "m:sys".to_string(),
                parent_key: None,
                name: "sys".to_string(),
            },
            ImportGroupSpec {
                key: "r:user".to_string(),
                parent_key: Some("m:sys".to_string()),
                name: "user".to_string(),
            },
        ]
    }

    fn request_spec() -> ImportRequestSpec {
        ImportRequestSpec {
            group_key: "r:user".to_string(),
            external_key: Some("smp/sys/user#find_page".to_string()),
            name: "user · find_page".to_string(),
            method: "POST".to_string(),
            url: "/api".to_string(),
            headers: vec![KeyValueRow::new("Authorization", "Bearer {{token}}")],
            body: RequestBody {
                kind: BodyKind::Json,
                text: "{\"resource\":\"smp/sys/user\"}".to_string(),
                form: Vec::new(),
            },
            timeout_ms: 30_000,
            ..Default::default()
        }
    }

    #[test]
    fn import_replaces_existing_groups_and_writes_external_key() {
        let (database, project_id) = seeded();
        let group_repo = ApiGroupRepository::new(&database);
        let request_repo = ApiRequestRepository::new(&database);

        // 预置手动分组与请求，导入时应按 replace 语义清空
        let old_group = group_repo.list(&project_id).expect("default group")[0].clone();
        let old_request = request_repo
            .create(&old_group.id, "手写请求")
            .expect("manual request");

        let summary = ApiImportRepository::new(&database)
            .run_import(&project_id, true, &module_spec(), &[request_spec()])
            .expect("import should succeed");

        assert_eq!(summary.created_groups, 2);
        assert_eq!(summary.created_requests, 1);
        assert_eq!(summary.deleted_groups, 1);
        assert_eq!(summary.deleted_requests, 1);

        assert!(request_repo.get(&old_request.id).expect("get").is_none());

        let groups = group_repo.list(&project_id).expect("groups");
        assert_eq!(groups.len(), 2);
        let module = groups
            .iter()
            .find(|group| group.name == "sys")
            .expect("module group");
        let resource = groups
            .iter()
            .find(|group| group.name == "user")
            .expect("resource group");
        assert_eq!(
            resource.parent_group_id.as_deref(),
            Some(module.id.as_str())
        );

        let requests = request_repo.list(&project_id).expect("requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].group_id, resource.id);
        assert_eq!(
            requests[0].external_key.as_deref(),
            Some("smp/sys/user#find_page")
        );
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].headers.len(), 1);
    }

    #[test]
    fn import_without_replace_keeps_manual_groups() {
        let (database, project_id) = seeded();
        let repo = ApiImportRepository::new(&database);

        let summary = repo
            .run_import(&project_id, false, &module_spec(), &[request_spec()])
            .expect("import should succeed");

        assert_eq!(summary.created_groups, 2);
        assert_eq!(summary.deleted_groups, 0);
        // 默认分组仍在
        assert_eq!(
            ApiGroupRepository::new(&database)
                .list(&project_id)
                .expect("groups")
                .len(),
            3
        );
    }

    #[test]
    fn import_rejects_unknown_parent_and_duplicate_keys() {
        let (database, project_id) = seeded();
        let repo = ApiImportRepository::new(&database);

        let mut specs = module_spec();
        specs[1].parent_key = Some("m:missing".to_string());
        let error = repo
            .run_import(&project_id, true, &specs, &[])
            .expect_err("unknown parent must be rejected");
        assert!(error.contains("不存在的父分组键"), "got {error}");

        let mut duplicated = module_spec();
        duplicated.push(ImportGroupSpec {
            key: "m:sys".to_string(),
            parent_key: None,
            name: "sys2".to_string(),
        });
        let error = repo
            .run_import(&project_id, true, &duplicated, &[])
            .expect_err("duplicate keys must be rejected");
        assert!(error.contains("分组键重复"), "got {error}");
    }

    #[test]
    fn import_rejects_request_with_unknown_group_and_bad_method() {
        let (database, project_id) = seeded();
        let repo = ApiImportRepository::new(&database);

        let mut request = request_spec();
        request.group_key = "r:missing".to_string();
        let error = repo
            .run_import(&project_id, true, &module_spec(), &[request])
            .expect_err("unknown group key must be rejected");
        assert!(error.contains("不存在的分组键"), "got {error}");

        let mut request = request_spec();
        request.method = "TRACE".to_string();
        let error = repo
            .run_import(&project_id, true, &module_spec(), &[request])
            .expect_err("bad method must be rejected");
        assert!(error.contains("不支持的 HTTP 方法"), "got {error}");
    }

    #[test]
    fn save_source_upserts_single_row_per_project() {
        let (database, project_id) = seeded();
        let repo = ApiImportRepository::new(&database);

        let first = repo
            .save_source(&project_id, "vef-go", "/tmp/smp-server")
            .expect("first save");
        let second = repo
            .save_source(&project_id, "vef-go", "/tmp/smp-server-v2")
            .expect("second save");

        assert_eq!(first.id, second.id);
        let sources = repo.list_sources(&project_id).expect("sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].repo_path, "/tmp/smp-server-v2");
    }
}

use crate::db::api_client::ApiClientDatabase;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::schema_dict_repo::{SchemaDatabaseRepository, SchemaSnapshotRepository};
use crate::models::schema_dict::{
    diff_schemas, normalize_db_type, normalize_source_kind, ParsedSchema, SchemaChange,
    SchemaDatabase, SchemaSnapshot, TimelineEntry,
};
use crate::services::schema_import;
use crate::services::schema_ai;

// ===== 库条目 =====

#[tauri::command]
pub fn list_schema_databases(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<SchemaDatabase>, String> {
    SchemaDatabaseRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_schema_database(
    database: tauri::State<'_, ApiClientDatabase>,
    item: SchemaDatabase,
) -> Result<SchemaDatabase, String> {
    SchemaDatabaseRepository::new(&database).upsert(&item)
}

#[tauri::command]
pub fn delete_schema_database(
    database: tauri::State<'_, ApiClientDatabase>,
    database_id: String,
) -> Result<String, String> {
    SchemaDatabaseRepository::new(&database).delete(&database_id)
}

// ===== 快照 =====

#[derive(Debug, serde::Deserialize)]
pub struct ImportSchemaSnapshotRequest {
    pub database_id: String,
    pub label: String,
    #[serde(default)]
    pub note: String,
    /// ddl | query
    pub source_kind: String,
    /// DDL 文本或查询结果粘贴内容
    pub source_text: String,
}

/// 干跑解析：只解析并返回预览，不落库
#[tauri::command]
pub fn parse_schema_input(
    database: tauri::State<'_, ApiClientDatabase>,
    db_type: String,
    source_kind: String,
    source_text: String,
) -> Result<ParsedSchema, String> {
    let db_type = normalize_db_type(&db_type);
    let source_kind = normalize_source_kind(&source_kind);
    parse_input(&db_type, &source_kind, &source_text).map(|outcome| outcome.0)
}

/// 导入快照：服务端重新解析后落库（不信任前端传来的解析结果）
#[tauri::command]
pub fn import_schema_snapshot(
    database: tauri::State<'_, ApiClientDatabase>,
    request: ImportSchemaSnapshotRequest,
) -> Result<SchemaSnapshot, String> {
    let repo = SchemaDatabaseRepository::new(&database);
    let target = repo
        .list()?
        .into_iter()
        .find(|item| item.id == request.database_id)
        .ok_or_else(|| format!("未找到 id 为 {} 的库条目", request.database_id))?;

    let source_kind = normalize_source_kind(&request.source_kind);
    let (schema, _skipped) = parse_input(
        &normalize_db_type(&target.db_type),
        &source_kind,
        &request.source_text,
    )?;

    let snapshot = SchemaSnapshot {
        database_id: target.id,
        label: request.label,
        source_kind,
        source_text: request.source_text,
        tables_json: serde_json::to_string(&schema)
            .map_err(|error| format!("序列化解析结果失败: {error}"))?,
        note: request.note,
        ..Default::default()
    };

    SchemaSnapshotRepository::new(&database).upsert(&snapshot)
}

#[tauri::command]
pub fn list_schema_snapshots(
    database: tauri::State<'_, ApiClientDatabase>,
    database_id: String,
) -> Result<Vec<SchemaSnapshot>, String> {
    SchemaSnapshotRepository::new(&database).list_by_database(&database_id)
}

#[tauri::command]
pub fn delete_schema_snapshot(
    database: tauri::State<'_, ApiClientDatabase>,
    snapshot_id: String,
) -> Result<String, String> {
    SchemaSnapshotRepository::new(&database).delete(&snapshot_id)
}

// ===== 时间线与 AI =====

/// 时间线：每个快照相对其上一版（按 created_at 排序）的变更列表
#[tauri::command]
pub fn get_schema_timeline(
    database: tauri::State<'_, ApiClientDatabase>,
    database_id: String,
) -> Result<Vec<TimelineEntry>, String> {
    let snapshots = SchemaSnapshotRepository::new(&database).list_by_database(&database_id)?;
    Ok(build_timeline(&snapshots))
}

/// AI 解读两个快照间的变更
#[tauri::command]
pub async fn interpret_schema_changes(
    store: tauri::State<'_, LocalJsonStore>,
    database: tauri::State<'_, ApiClientDatabase>,
    before_snapshot_id: String,
    after_snapshot_id: String,
) -> Result<String, String> {
    let snap_repo = SchemaSnapshotRepository::new(&database);
    let before = snap_repo
        .get(&before_snapshot_id)?
        .ok_or_else(|| format!("未找到 id 为 {before_snapshot_id} 的快照"))?;
    let after = snap_repo
        .get(&after_snapshot_id)?
        .ok_or_else(|| format!("未找到 id 为 {after_snapshot_id} 的快照"))?;

    let before_schema: ParsedSchema = serde_json::from_str(&before.tables_json)
        .map_err(|error| format!("快照数据损坏: {error}"))?;
    let after_schema: ParsedSchema = serde_json::from_str(&after.tables_json)
        .map_err(|error| format!("快照数据损坏: {error}"))?;

    let changes = diff_schemas(&before_schema, &after_schema);
    if changes.is_empty() {
        return Ok("这两个快照的表结构完全一致，没有变化。".to_string());
    }

    let entry = TimelineEntry {
        snapshot_id: after.id,
        label: after.label,
        source_kind: after.source_kind,
        created_at: after.created_at,
        changes: Vec::new(),
    };
    schema_ai::interpret_changes(&store, &before.label, &entry, &changes).await
}

/// AI 针对指定快照的表结构问答
#[tauri::command]
pub async fn ask_schema_dict(
    store: tauri::State<'_, LocalJsonStore>,
    database: tauri::State<'_, ApiClientDatabase>,
    snapshot_id: String,
    question: String,
) -> Result<String, String> {
    let db_repo = SchemaDatabaseRepository::new(&database);
    let snap_repo = SchemaSnapshotRepository::new(&database);

    let snapshot = snap_repo
        .get(&snapshot_id)?
        .ok_or_else(|| format!("未找到 id 为 {snapshot_id} 的快照"))?;
    let target = db_repo
        .list()?
        .into_iter()
        .find(|item| item.id == snapshot.database_id)
        .ok_or("快照所属的库条目不存在")?;

    schema_ai::answer_question(&store, &target.name, &snapshot.tables_json, &question).await
}

// ===== 内部 =====

type ParseOutcome = (ParsedSchema, usize);

fn parse_input(db_type: &str, source_kind: &str, source_text: &str) -> Result<ParseOutcome, String> {
    match source_kind {
        "query" => schema_import::parse_query_result(source_text).map(|schema| (schema, 0)),
        _ => schema_import::parse_ddl(source_text, db_type)
            .map(|outcome| (outcome.schema, outcome.skipped_statements)),
    }
}

fn build_timeline(snapshots: &[SchemaSnapshot]) -> Vec<TimelineEntry> {
    let mut entries = Vec::with_capacity(snapshots.len());
    for (index, snapshot) in snapshots.iter().enumerate() {
        let changes: Vec<SchemaChange> = if index == 0 {
            Vec::new()
        } else {
            let before = &snapshots[index - 1];
            match (
                serde_json::from_str::<ParsedSchema>(&before.tables_json),
                serde_json::from_str::<ParsedSchema>(&snapshot.tables_json),
            ) {
                (Ok(before_schema), Ok(after_schema)) => diff_schemas(&before_schema, &after_schema),
                _ => Vec::new(),
            }
        };

        entries.push(TimelineEntry {
            snapshot_id: snapshot.id.clone(),
            label: snapshot.label.clone(),
            source_kind: snapshot.source_kind.clone(),
            created_at: snapshot.created_at,
            changes,
        });
    }
    entries
}

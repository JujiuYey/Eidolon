//! 数据字典 DTO 与 diff。
//!
//! 表结构快照（对应 `schema_databases` / `schema_snapshots` 表）。
//! 解析产物 `ParsedSchema` 以 JSON 存进 `schema_snapshots.tables_json`，
//! 变更时间线由相邻快照两两 diff 得出。

use serde::{Deserialize, Serialize};

/// 数据字典 AI 功能使用的默认模型 key
pub const SCHEMA_ASSISTANT_MODEL_KEY: &str = "schema_assistant";

pub const DB_TYPE_MYSQL: &str = "mysql";
pub const DB_TYPE_PG: &str = "pg";
pub const DB_TYPE_ORACLE: &str = "oracle";

pub const SOURCE_KIND_DDL: &str = "ddl";
pub const SOURCE_KIND_QUERY: &str = "query";

/// 归一化方言：非法值回落为 mysql
pub fn normalize_db_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        DB_TYPE_PG => DB_TYPE_PG.to_string(),
        DB_TYPE_ORACLE => DB_TYPE_ORACLE.to_string(),
        _ => DB_TYPE_MYSQL.to_string(),
    }
}

/// 归一化导入来源：非法值回落为 ddl
pub fn normalize_source_kind(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        SOURCE_KIND_QUERY => SOURCE_KIND_QUERY.to_string(),
        _ => SOURCE_KIND_DDL.to_string(),
    }
}

// ===== 库条目 DTO =====

/// 库条目（对应 `schema_databases` 表）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaDatabase {
    #[serde(default)]
    pub id: String,
    /// 显示名（如 "病案质控库"）
    #[serde(default)]
    pub name: String,
    /// mysql | pg | oracle
    #[serde(default)]
    pub db_type: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub sort: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

// ===== 快照 DTO =====

/// 快照（对应 `schema_snapshots` 表）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SchemaSnapshot {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub database_id: String,
    /// 版本名（如 "2026-10 升级前"）
    #[serde(default)]
    pub label: String,
    /// ddl | query
    #[serde(default)]
    pub source_kind: String,
    /// 原始粘贴文本
    #[serde(default)]
    pub source_text: String,
    /// 解析后的结构化 JSON（ParsedSchema）
    #[serde(default)]
    pub tables_json: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

// ===== 解析产物（存 tables_json）=====

/// 解析后的完整表结构
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedSchema {
    #[serde(default)]
    pub tables: Vec<ParsedTable>,
    /// 解析时跳过的无法识别语句条数（DDL 导入时产生）
    #[serde(default)]
    pub skipped_statements: i64,
}

/// 一张表
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedTable {
    pub name: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub columns: Vec<ParsedColumn>,
    #[serde(default)]
    pub indexes: Vec<ParsedIndex>,
}

/// 一列
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedColumn {
    pub name: String,
    /// 类型显示串（如 varchar(255)、number(10,2)）
    pub data_type: String,
    #[serde(default = "default_true")]
    pub nullable: bool,
    /// 默认值显示串；无默认为 None
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub primary_key: bool,
    /// 查询结果导入时的列序号，仅解析中转用，落库前清空
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<i64>,
}

/// 一个索引
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedIndex {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub unique: bool,
}

fn default_true() -> bool {
    true
}

// ===== 变更时间线 =====

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaChangeKind {
    /// 新增表
    TableAdded,
    /// 删除表
    TableRemoved,
    /// 新增字段
    ColumnAdded,
    /// 删除字段
    ColumnRemoved,
    /// 类型变更
    ColumnTypeChanged,
    /// 可空性变更
    NullableChanged,
    /// 默认值变更
    DefaultChanged,
    /// 注释变更
    CommentChanged,
    /// 主键变更
    PrimaryKeyChanged,
}

/// 一条结构变更
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaChange {
    pub kind: SchemaChangeKind,
    /// 涉及的表名
    pub table: String,
    /// 涉及的字段名（表级变更时为 None）
    pub column: Option<String>,
    /// 旧值（人类可读）
    #[serde(default)]
    pub from: String,
    /// 新值（人类可读）
    #[serde(default)]
    pub to: String,
}

/// 时间线的一项：某快照相对其上一版快照的全部变更
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntry {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub created_at: i64,
    /// 相对上一版的变更；首个快照此列表为空
    #[serde(default)]
    pub changes: Vec<SchemaChange>,
}

/// 表名 → 显示名（多段名取最后一段，统一小写用于展示排序无关的比较由调用方决定）
pub fn display_table_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
}

/// 比较两份解析结果，产出变更列表（按表名精确匹配；索引不参与 diff）
pub fn diff_schemas(before: &ParsedSchema, after: &ParsedSchema) -> Vec<SchemaChange> {
    let mut changes = Vec::new();
    let before_map: std::collections::HashMap<&str, &ParsedTable> = before
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect();
    let after_map: std::collections::HashMap<&str, &ParsedTable> = after
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect();

    // 新增 / 变更的表
    for table in &after.tables {
        match before_map.get(table.name.as_str()) {
            None => changes.push(SchemaChange {
                kind: SchemaChangeKind::TableAdded,
                table: table.name.clone(),
                column: None,
                from: String::new(),
                to: format!("{} 个字段", table.columns.len()),
            }),
            Some(old_table) => changes.extend(diff_table(old_table, table)),
        }
    }

    // 删除的表
    for table in &before.tables {
        if !after_map.contains_key(table.name.as_str()) {
            changes.push(SchemaChange {
                kind: SchemaChangeKind::TableRemoved,
                table: table.name.clone(),
                column: None,
                from: format!("{} 个字段", table.columns.len()),
                to: String::new(),
            });
        }
    }

    changes
}

fn diff_table(before: &ParsedTable, after: &ParsedTable) -> Vec<SchemaChange> {
    let mut changes = Vec::new();
    let before_columns: std::collections::HashMap<&str, &ParsedColumn> = before
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect();
    let after_columns: std::collections::HashMap<&str, &ParsedColumn> = after
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect();

    for column in &after.columns {
        match before_columns.get(column.name.as_str()) {
            None => changes.push(SchemaChange {
                kind: SchemaChangeKind::ColumnAdded,
                table: after.name.clone(),
                column: Some(column.name.clone()),
                from: String::new(),
                to: column.data_type.clone(),
            }),
            Some(old) => changes.extend(diff_column(after.name.as_str(), old, column)),
        }
    }

    for column in &before.columns {
        if !after_columns.contains_key(column.name.as_str()) {
            changes.push(SchemaChange {
                kind: SchemaChangeKind::ColumnRemoved,
                table: after.name.clone(),
                column: Some(column.name.clone()),
                from: column.data_type.clone(),
                to: String::new(),
            });
        }
    }

    if before.comment != after.comment {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::CommentChanged,
            table: after.name.clone(),
            column: None,
            from: before.comment.clone(),
            to: after.comment.clone(),
        });
    }

    let before_pk: Vec<&str> = before.columns.iter().filter(|c| c.primary_key).map(|c| c.name.as_str()).collect();
    let after_pk: Vec<&str> = after.columns.iter().filter(|c| c.primary_key).map(|c| c.name.as_str()).collect();
    if before_pk != after_pk {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::PrimaryKeyChanged,
            table: after.name.clone(),
            column: None,
            from: before_pk.join(", "),
            to: after_pk.join(", "),
        });
    }

    changes
}

fn diff_column(table: &str, before: &ParsedColumn, after: &ParsedColumn) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    if before.data_type != after.data_type {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::ColumnTypeChanged,
            table: table.to_string(),
            column: Some(after.name.clone()),
            from: before.data_type.clone(),
            to: after.data_type.clone(),
        });
    }
    if before.nullable != after.nullable {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::NullableChanged,
            table: table.to_string(),
            column: Some(after.name.clone()),
            from: if before.nullable { "可空" } else { "非空" }.to_string(),
            to: if after.nullable { "可空" } else { "非空" }.to_string(),
        });
    }
    if before.default != after.default {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::DefaultChanged,
            table: table.to_string(),
            column: Some(after.name.clone()),
            from: before.default.clone().unwrap_or_default(),
            to: after.default.clone().unwrap_or_default(),
        });
    }
    if before.comment != after.comment {
        changes.push(SchemaChange {
            kind: SchemaChangeKind::CommentChanged,
            table: table.to_string(),
            column: Some(after.name.clone()),
            from: before.comment.clone(),
            to: after.comment.clone(),
        });
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str, columns: Vec<ParsedColumn>) -> ParsedTable {
        ParsedTable {
            name: name.to_string(),
            comment: String::new(),
            columns,
            indexes: Vec::new(),
        }
    }

    fn column(name: &str, data_type: &str) -> ParsedColumn {
        ParsedColumn {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable: true,
            default: None,
            comment: String::new(),
            primary_key: false,
            ordinal: None,
        }
    }

    #[test]
    fn diff_reports_added_removed_and_changed() {
        let before = ParsedSchema {
            tables: vec![
                table(
                    "t_patient",
                    vec![column("id", "int"), column("name", "varchar(50)")],
                ),
                table("t_old", vec![column("id", "int")]),
            ],
            skipped_statements: 0,
        };
        let after = ParsedSchema {
            tables: vec![table(
                "t_patient",
                vec![
                    ParsedColumn {
                        data_type: "varchar(100)".to_string(),
                        ..column("name", "")
                    },
                    column("id", "int"),
                    column("id_card", "varchar(18)"),
                ],
            )],
            skipped_statements: 0,
        };

        let changes = diff_schemas(&before, &after);
        let kinds: Vec<_> = changes.iter().map(|change| (change.kind, change.table.as_str(), change.column.as_deref())).collect();

        assert!(kinds.contains(&(SchemaChangeKind::TableRemoved, "t_old", None)));
        assert!(kinds.contains(&(SchemaChangeKind::ColumnAdded, "t_patient", Some("id_card"))));
        assert!(kinds.contains(&(SchemaChangeKind::ColumnTypeChanged, "t_patient", Some("name"))));
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn identical_schemas_produce_no_changes() {
        let schema = ParsedSchema {
            tables: vec![table("t_a", vec![column("id", "int")])],
            skipped_statements: 0,
        };
        assert!(diff_schemas(&schema, &schema).is_empty());
    }

    #[test]
    fn normalize_falls_back_safely() {
        assert_eq!(normalize_db_type(" PG "), "pg");
        assert_eq!(normalize_db_type("unknown"), "mysql");
        assert_eq!(normalize_source_kind("query"), "query");
        assert_eq!(normalize_source_kind(""), "ddl");
    }
}

export type SchemaDbType = 'mysql' | 'pg' | 'oracle';

export type SchemaSourceKind = 'ddl' | 'query';

export interface SchemaDatabase {
  id: string;
  name: string;
  db_type: SchemaDbType;
  note: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

export interface SchemaSnapshot {
  id: string;
  database_id: string;
  label: string;
  source_kind: SchemaSourceKind;
  source_text: string;
  tables_json: string;
  note: string;
  created_at: number;
  updated_at: number;
}

// ===== 解析产物（对应后端 ParsedSchema）=====

export interface ParsedSchema {
  tables: ParsedTable[];
  skipped_statements: number;
}

export interface ParsedTable {
  name: string;
  comment: string;
  columns: ParsedColumn[];
  indexes: ParsedIndex[];
}

export interface ParsedColumn {
  name: string;
  data_type: string;
  nullable: boolean;
  default: string | null;
  comment: string;
  primary_key: boolean;
}

export interface ParsedIndex {
  name: string;
  columns: string[];
  unique: boolean;
}

// ===== 变更时间线 =====

export type SchemaChangeKind =
  | 'table_added'
  | 'table_removed'
  | 'column_added'
  | 'column_removed'
  | 'column_type_changed'
  | 'nullable_changed'
  | 'default_changed'
  | 'comment_changed'
  | 'primary_key_changed';

export interface SchemaChange {
  kind: SchemaChangeKind;
  table: string;
  column: string | null;
  from: string;
  to: string;
}

export interface TimelineEntry {
  snapshot_id: string;
  label: string;
  source_kind: SchemaSourceKind;
  created_at: number;
  changes: SchemaChange[];
}

/** 导入快照的请求体 */
export interface ImportSchemaSnapshotRequest {
  database_id: string;
  label: string;
  note: string;
  source_kind: SchemaSourceKind;
  source_text: string;
}

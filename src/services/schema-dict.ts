import { invoke } from '@tauri-apps/api/core';

import type {
  ImportSchemaSnapshotRequest,
  ParsedSchema,
  SchemaDatabase,
  SchemaSnapshot,
  TimelineEntry,
} from '@/types/schema-dict';

// ===== 库条目 =====

export async function listSchemaDatabases(): Promise<SchemaDatabase[]> {
  return invoke<SchemaDatabase[]>('list_schema_databases');
}

export async function upsertSchemaDatabase(item: SchemaDatabase): Promise<SchemaDatabase> {
  return invoke<SchemaDatabase>('upsert_schema_database', { item });
}

export async function deleteSchemaDatabase(databaseId: string): Promise<string> {
  return invoke<string>('delete_schema_database', { databaseId });
}

// ===== 快照 =====

export async function listSchemaSnapshots(databaseId: string): Promise<SchemaSnapshot[]> {
  return invoke<SchemaSnapshot[]>('list_schema_snapshots', { databaseId });
}

export async function deleteSchemaSnapshot(snapshotId: string): Promise<string> {
  return invoke<string>('delete_schema_snapshot', { snapshotId });
}

/** 干跑解析：只预览不落库 */
export async function parseSchemaInput(dbType: string, sourceKind: string, sourceText: string): Promise<ParsedSchema> {
  return invoke<ParsedSchema>('parse_schema_input', { dbType, sourceKind, sourceText });
}

export async function importSchemaSnapshot(request: ImportSchemaSnapshotRequest): Promise<SchemaSnapshot> {
  return invoke<SchemaSnapshot>('import_schema_snapshot', { request });
}

export async function getSchemaTimeline(databaseId: string): Promise<TimelineEntry[]> {
  return invoke<TimelineEntry[]>('get_schema_timeline', { databaseId });
}

// ===== AI =====

/** AI 解读两个快照间的变更 */
export async function interpretSchemaChanges(beforeSnapshotId: string, afterSnapshotId: string): Promise<string> {
  return invoke<string>('interpret_schema_changes', { beforeSnapshotId, afterSnapshotId });
}

/** AI 针对指定快照的表结构问答 */
export async function askSchemaDict(snapshotId: string, question: string): Promise<string> {
  return invoke<string>('ask_schema_dict', { snapshotId, question });
}

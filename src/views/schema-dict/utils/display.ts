/**
 * 数据字典页展示辅助函数（纯函数，可单测）。
 *
 * 变更类型的文案/样式、排序、表搜索过滤、表名排序。
 */

import type {
  ParsedSchema,
  ParsedTable,
  SchemaChange,
  SchemaChangeKind,
  SchemaDatabase,
  SchemaSnapshot,
} from '@/types/schema-dict';

/** 变更类型 → 文案 */
export const CHANGE_KIND_LABELS: Record<SchemaChangeKind, string> = {
  table_added: '新增表',
  table_removed: '删除表',
  column_added: '新增字段',
  column_removed: '删除字段',
  column_type_changed: '类型变更',
  nullable_changed: '可空性变更',
  default_changed: '默认值变更',
  comment_changed: '注释变更',
  primary_key_changed: '主键变更',
};

export function changeKindLabel(kind: SchemaChangeKind): string {
  return CHANGE_KIND_LABELS[kind] ?? kind;
}

/** 变更的性质：破坏性（红）/ 新增（绿）/ 中性（灰） */
export type ChangeTone = 'danger' | 'added' | 'neutral';

export function changeTone(kind: SchemaChangeKind): ChangeTone {
  if (kind === 'table_removed' || kind === 'column_removed') {
    return 'danger';
  }
  if (kind === 'table_added' || kind === 'column_added') {
    return 'added';
  }
  return 'neutral';
}

export const CHANGE_TONE_CLASSES: Record<ChangeTone, string> = {
  danger: 'bg-destructive/10 text-destructive',
  added: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  neutral: 'bg-muted text-muted-foreground',
};

const KIND_ORDER: SchemaChangeKind[] = [
  'table_removed',
  'table_added',
  'column_removed',
  'column_added',
  'column_type_changed',
  'nullable_changed',
  'default_changed',
  'primary_key_changed',
  'comment_changed',
];

/** 变更排序：先表级后字段级，同级按类型顺序、再按表名字母序 */
export function sortChanges(changes: SchemaChange[]): SchemaChange[] {
  return [...changes].sort((a, b) => {
    const kindOrder = KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind);
    if (kindOrder !== 0) {
      return kindOrder;
    }
    const tableOrder = a.table.localeCompare(b.table);
    if (tableOrder !== 0) {
      return tableOrder;
    }
    return (a.column ?? '').localeCompare(b.column ?? '');
  });
}

/** 表名按字母序（大小写不敏感） */
export function sortTables(tables: ParsedTable[]): ParsedTable[] {
  return [...tables].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
}

/** 按关键词过滤表（匹配表名或表注释，大小写不敏感） */
export function filterTables(tables: ParsedTable[], keyword: string): ParsedTable[] {
  const query = keyword.trim().toLowerCase();
  if (!query) {
    return tables;
  }
  return tables.filter(
    table => table.name.toLowerCase().includes(query) || table.comment.toLowerCase().includes(query),
  );
}

/** 快照结构的摘要：N 张表 / M 个字段 / 跳过语句提示 */
export function schemaSummary(schema: ParsedSchema): string {
  const tableCount = schema.tables.length;
  const columnCount = schema.tables.reduce((sum, table) => sum + table.columns.length, 0);
  const base = `${tableCount} 张表 · ${columnCount} 个字段`;
  return schema.skipped_statements > 0 ? `${base} · 跳过 ${schema.skipped_statements} 条无法识别的语句` : base;
}

/** 字段类型截断显示（number(38,0) 这类保持原样，超长的截断） */
export function formatDataType(dataType: string, maxChars = 40): string {
  if (dataType.length <= maxChars) {
    return dataType;
  }
  return `${dataType.slice(0, maxChars)}…`;
}

/** 单条变更的人话描述（时间线用） */
export function describeChange(change: SchemaChange): string {
  const where = change.column ? `${change.table}.${change.column}` : change.table;
  switch (change.kind) {
    case 'table_added':
      return `新增表 ${change.table}（${change.to}）`;
    case 'table_removed':
      return `删除表 ${change.table}（原来 ${change.from}）`;
    case 'column_added':
      return `${where} 新增（${change.to}）`;
    case 'column_removed':
      return `${where} 删除（原来 ${change.from}）`;
    case 'column_type_changed':
      return `${where} 类型 ${change.from} → ${change.to}`;
    case 'nullable_changed':
      return `${where} ${change.from} → ${change.to}`;
    case 'default_changed':
      return `${where} 默认值 ${change.from || '（无）'} → ${change.to || '（无）'}`;
    case 'comment_changed':
      return change.column ? `${where} 注释更新` : `${change.table} 表注释更新`;
    case 'primary_key_changed':
      return `${change.table} 主键 ${change.from || '（无）'} → ${change.to || '（无）'}`;
    default:
      return where;
  }
}

/** 数据字典统计瓦片 */
export interface SchemaStats {
  databaseCount: number;
  /** 当前库的快照数（快照按库加载） */
  snapshotCount: number;
  /** 当前库最新快照覆盖的表数 */
  tableCount: number;
  /** 当前库最近一次快照时间，无快照为 0 */
  lastSnapshotAt: number;
}

/** 库列表 + 当前库快照 → 统计瓦片数据 */
export function computeSchemaStats(
  databases: SchemaDatabase[],
  snapshots: SchemaSnapshot[],
  tableCount: number,
): SchemaStats {
  return {
    databaseCount: databases.length,
    snapshotCount: snapshots.length,
    tableCount,
    lastSnapshotAt: snapshots.reduce((latest, snapshot) => Math.max(latest, snapshot.created_at), 0),
  };
}

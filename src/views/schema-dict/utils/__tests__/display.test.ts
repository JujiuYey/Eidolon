import { describe, expect, it } from 'vitest';

import type { ParsedTable, SchemaChange, SchemaDatabase, SchemaSnapshot } from '@/types/schema-dict';

import {
  changeKindLabel,
  changeTone,
  computeSchemaStats,
  describeChange,
  filterTables,
  formatDataType,
  schemaSummary,
  sortChanges,
  sortTables,
} from '../display';

function makeChange(overrides: Partial<SchemaChange> = {}): SchemaChange {
  return {
    kind: 'column_added',
    table: 't_a',
    column: 'c1',
    from: '',
    to: 'int',
    ...overrides,
  };
}

function makeTable(overrides: Partial<ParsedTable> = {}): ParsedTable {
  return {
    name: 't_a',
    comment: '',
    columns: [],
    indexes: [],
    ...overrides,
  };
}

function makeColumn(
  overrides: Partial<import('@/types/schema-dict').ParsedColumn> = {},
): import('@/types/schema-dict').ParsedColumn {
  return {
    name: 'c1',
    data_type: 'int',
    nullable: true,
    default: null,
    comment: '',
    primary_key: false,
    ...overrides,
  };
}

describe('schema dict display helpers', () => {
  it('labels and tones change kinds', () => {
    expect(changeKindLabel('table_removed')).toBe('删除表');
    expect(changeTone('table_removed')).toBe('danger');
    expect(changeTone('table_added')).toBe('added');
    expect(changeTone('column_type_changed')).toBe('neutral');
  });

  it('sorts changes: table level first, then by table and column name', () => {
    const sorted = sortChanges([
      makeChange({ kind: 'column_added', table: 't_b', column: 'c2' }),
      makeChange({ kind: 'table_added', table: 't_z' }),
      makeChange({ kind: 'column_removed', table: 't_b', column: 'c1' }),
      makeChange({ kind: 'column_added', table: 't_a', column: 'c9' }),
      makeChange({ kind: 'table_removed', table: 't_a' }),
    ]);

    expect(sorted.map(change => change.kind)).toEqual([
      'table_removed',
      'table_added',
      'column_removed',
      'column_added',
      'column_added',
    ]);
    expect(sorted[3]?.table).toBe('t_a');
  });

  it('sorts tables case-insensitively', () => {
    const sorted = sortTables([makeTable({ name: 'Visit' }), makeTable({ name: 'patient' })]);
    expect(sorted.map(table => table.name)).toEqual(['patient', 'Visit']);
  });

  it('filters tables by name or comment', () => {
    const tables = [
      makeTable({ name: 'visit_record', comment: '就诊记录' }),
      makeTable({ name: 'patient', comment: '病人' }),
    ];
    expect(filterTables(tables, 'VISIT')).toHaveLength(1);
    expect(filterTables(tables, '就诊')).toEqual([tables[0]]);
    expect(filterTables(tables, '')).toHaveLength(2);
  });

  it('summarizes schema with skipped statements', () => {
    const schema = {
      tables: [
        makeTable({ columns: [makeColumn(), makeColumn(), makeColumn()] }),
        makeTable({ columns: [makeColumn()] }),
      ],
      skipped_statements: 3,
    };
    expect(schemaSummary(schema)).toBe('2 张表 · 4 个字段 · 跳过 3 条无法识别的语句');

    expect(schemaSummary({ tables: [], skipped_statements: 0 })).toBe('0 张表 · 0 个字段');
  });

  it('truncates long data types', () => {
    expect(formatDataType('int')).toBe('int');
    expect(formatDataType('a'.repeat(50))).toHaveLength(41);
    expect(formatDataType('a'.repeat(50))).toMatch(/…$/);
  });

  it('describes changes in plain language', () => {
    expect(describeChange(makeChange({ kind: 'table_added', table: 't_x', to: '5 个字段' }))).toBe(
      '新增表 t_x（5 个字段）',
    );
    expect(describeChange(makeChange({ kind: 'column_type_changed', from: 'int', to: 'bigint' }))).toBe(
      't_a.c1 类型 int → bigint',
    );
    expect(describeChange(makeChange({ kind: 'default_changed', from: '', to: '0' }))).toBe('t_a.c1 默认值 （无） → 0');
    expect(describeChange(makeChange({ kind: 'comment_changed', column: null }))).toBe('t_a 表注释更新');
  });

  it('computes schema stats from databases and current-database snapshots', () => {
    const databases = [
      { id: 'db_1', name: '病案', db_type: 'mysql', note: '', sort: 0, created_at: 1, updated_at: 1 },
      { id: 'db_2', name: '体检', db_type: 'mysql', note: '', sort: 0, created_at: 1, updated_at: 1 },
    ] as SchemaDatabase[];
    const snapshots = [
      {
        id: 's1',
        database_id: 'db_1',
        label: 'v1',
        source_kind: 'ddl',
        source_text: '',
        tables_json: '',
        note: '',
        created_at: 100,
        updated_at: 100,
      },
      {
        id: 's2',
        database_id: 'db_1',
        label: 'v2',
        source_kind: 'ddl',
        source_text: '',
        tables_json: '',
        note: '',
        created_at: 300,
        updated_at: 300,
      },
    ] as SchemaSnapshot[];

    const stats = computeSchemaStats(databases, snapshots, 42);

    expect(stats).toEqual({ databaseCount: 2, snapshotCount: 2, tableCount: 42, lastSnapshotAt: 300 });
  });

  it('schema stats tolerate empty state', () => {
    expect(computeSchemaStats([], [], 0)).toEqual({
      databaseCount: 0,
      snapshotCount: 0,
      tableCount: 0,
      lastSnapshotAt: 0,
    });
  });
});

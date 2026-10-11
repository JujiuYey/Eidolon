import { createPinia, defineStore, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createSchemaDictStore } from '@/stores/schema-dict';
import type { SchemaDictService } from '@/stores/schema-dict';
import type { ParsedSchema, SchemaDatabase, SchemaSnapshot, TimelineEntry } from '@/types/schema-dict';

function makeDatabase(overrides: Partial<SchemaDatabase> = {}): SchemaDatabase {
  return {
    id: 'sdb_1',
    name: '病案质控库',
    db_type: 'pg',
    note: '',
    sort: 0,
    created_at: 100,
    updated_at: 100,
    ...overrides,
  };
}

function makeSnapshot(overrides: Partial<SchemaSnapshot> = {}): SchemaSnapshot {
  return {
    id: 'ssn_1',
    database_id: 'sdb_1',
    label: '第一版',
    source_kind: 'ddl',
    source_text: '--',
    tables_json: JSON.stringify({
      tables: [{ name: 't_patient', comment: '', columns: [], indexes: [] }],
      skipped_statements: 0,
    }),
    note: '',
    created_at: 100,
    updated_at: 100,
    ...overrides,
  };
}

function makeTimeline(entries: Partial<TimelineEntry>[] = []): TimelineEntry[] {
  return entries.map((entry, index) => ({
    snapshot_id: `ssn_${index + 1}`,
    label: `v${index + 1}`,
    source_kind: 'ddl',
    created_at: 100 + index,
    changes: [],
    ...entry,
  }));
}

function createService(overrides: Partial<SchemaDictService> = {}): SchemaDictService {
  return {
    listSchemaDatabases: vi.fn().mockResolvedValue([]),
    upsertSchemaDatabase: vi
      .fn()
      .mockImplementation(async (item: SchemaDatabase) => ({ ...item, id: item.id || 'sdb_new' })),
    deleteSchemaDatabase: vi.fn().mockResolvedValue('sdb_1'),
    listSchemaSnapshots: vi.fn().mockResolvedValue([]),
    deleteSchemaSnapshot: vi.fn().mockResolvedValue('ssn_1'),
    parseSchemaInput: vi.fn().mockResolvedValue({ tables: [], skipped_statements: 0 } satisfies ParsedSchema),
    importSchemaSnapshot: vi.fn().mockImplementation(async request => ({
      ...makeSnapshot(),
      database_id: request.database_id,
      label: request.label,
    })),
    getSchemaTimeline: vi.fn().mockResolvedValue([]),
    interpretSchemaChanges: vi.fn().mockResolvedValue('这次主要是新增了结算相关表'),
    askSchemaDict: vi.fn().mockResolvedValue('住院记录存在 visit 表'),
    ...overrides,
  };
}

let storeSeq = 0;

function bindStore(service: SchemaDictService) {
  storeSeq += 1;
  const useBound = defineStore(`schema-dict-test-${storeSeq}`, () => createSchemaDictStore({ service }));
  return useBound();
}

describe('schema dict store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loadDatabases selects the first database and loads its data', async () => {
    const service = createService({
      listSchemaDatabases: vi.fn().mockResolvedValue([makeDatabase()]),
      listSchemaSnapshots: vi.fn().mockResolvedValue([makeSnapshot()]),
      getSchemaTimeline: vi.fn().mockResolvedValue(makeTimeline([{ changes: [] }])),
    });
    const store = bindStore(service);

    await store.loadDatabases();

    expect(store.activeDatabaseId).toBe('sdb_1');
    expect(store.snapshots).toHaveLength(1);
    expect(store.activeSnapshotId).toBe('ssn_1');
    expect(store.timeline).toHaveLength(1);
  });

  it('activeTables parses tables_json and survives corrupt data', async () => {
    const service = createService({
      listSchemaDatabases: vi.fn().mockResolvedValue([makeDatabase()]),
      listSchemaSnapshots: vi.fn().mockResolvedValue([makeSnapshot()]),
    });
    const store = bindStore(service);
    await store.loadDatabases();

    expect(store.activeTables.map(table => table.name)).toEqual(['t_patient']);

    store.snapshots[0]!.tables_json = '{ broken';
    expect(store.activeTables).toEqual([]);
  });

  it('saveDatabase adds a new database and selects it', async () => {
    const service = createService();
    const store = bindStore(service);

    const saved = await store.saveDatabase(makeDatabase({ id: '', name: '体检库', db_type: 'oracle' }));

    expect(saved.id).toBe('sdb_new');
    expect(store.activeDatabaseId).toBe('sdb_new');
    expect(service.listSchemaSnapshots).toHaveBeenCalledWith('sdb_new');
  });

  it('removeDatabase switches to the next database and reloads', async () => {
    const service = createService({
      listSchemaDatabases: vi.fn().mockResolvedValue([makeDatabase(), makeDatabase({ id: 'sdb_2', name: '体检库' })]),
    });
    const store = bindStore(service);
    await store.loadDatabases();

    await store.removeDatabase('sdb_1');

    expect(store.activeDatabaseId).toBe('sdb_2');
    expect(service.listSchemaSnapshots).toHaveBeenCalledWith('sdb_2');
  });

  it('previewParse records ready and error states without throwing', async () => {
    const service = createService({ parseSchemaInput: vi.fn().mockRejectedValue('解析失败') });
    const store = bindStore(service);

    const pending = store.previewParse('pg', 'ddl', 'CREATE TABLE t(id int)');
    expect(store.parsePreview?.status).toBe('parsing');
    await pending;
    expect(store.parsePreview?.status).toBe('error');
    expect(store.parsePreview?.errorMessage).toBe('解析失败');

    const ready = createService();
    const readyStore = bindStore(ready);
    await readyStore.previewParse('pg', 'ddl', 'CREATE TABLE t(id int)');
    expect(readyStore.parsePreview?.status).toBe('ready');
  });

  it('importSnapshot saves, reloads and selects the new snapshot', async () => {
    const newSnapshot = makeSnapshot({ id: 'ssn_2', label: '第二版' });
    const service = createService({
      listSchemaDatabases: vi.fn().mockResolvedValue([makeDatabase()]),
      listSchemaSnapshots: vi
        .fn()
        .mockResolvedValueOnce([makeSnapshot()])
        .mockResolvedValueOnce([makeSnapshot(), newSnapshot]),
      importSchemaSnapshot: vi.fn().mockResolvedValue(newSnapshot),
    });
    const store = bindStore(service);
    await store.loadDatabases();

    const saved = await store.importSnapshot({
      database_id: 'sdb_1',
      label: '第二版',
      note: '',
      source_kind: 'ddl',
      source_text: 'CREATE TABLE ...',
    });

    expect(saved.id).toBe('ssn_2');
    expect(store.activeSnapshotId).toBe('ssn_2');
    expect(store.snapshots).toHaveLength(2);
    expect(store.parsePreview).toBeNull();
  });

  it('interpretChanges and askQuestion store AI results and errors', async () => {
    const service = createService();
    const store = bindStore(service);
    store.activeSnapshotId = 'ssn_1';

    await store.interpretChanges('ssn_1', 'ssn_2');
    expect(store.aiInterpret?.status).toBe('ready');
    expect(store.aiInterpret?.text).toContain('结算');

    store.dismissInterpret();
    expect(store.aiInterpret).toBeNull();

    await store.askQuestion('住院记录在哪');
    expect(store.aiAnswer?.status).toBe('ready');
    expect(store.aiAnswer?.text).toBe('住院记录存在 visit 表');

    store.clearAnswer();
    expect(store.aiAnswer).toBeNull();
  });
});

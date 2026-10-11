import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import * as schemaDictServiceApi from '@/services/schema-dict';
import type {
  ImportSchemaSnapshotRequest,
  ParsedSchema,
  ParsedTable,
  SchemaDatabase,
  SchemaSnapshot,
  TimelineEntry,
} from '@/types/schema-dict';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface SchemaDictService {
  listSchemaDatabases: () => Promise<SchemaDatabase[]>;
  upsertSchemaDatabase: (item: SchemaDatabase) => Promise<SchemaDatabase>;
  deleteSchemaDatabase: (databaseId: string) => Promise<string>;
  listSchemaSnapshots: (databaseId: string) => Promise<SchemaSnapshot[]>;
  deleteSchemaSnapshot: (snapshotId: string) => Promise<string>;
  parseSchemaInput: (dbType: string, sourceKind: string, sourceText: string) => Promise<ParsedSchema>;
  importSchemaSnapshot: (request: ImportSchemaSnapshotRequest) => Promise<SchemaSnapshot>;
  getSchemaTimeline: (databaseId: string) => Promise<TimelineEntry[]>;
  interpretSchemaChanges: (beforeSnapshotId: string, afterSnapshotId: string) => Promise<string>;
  askSchemaDict: (snapshotId: string, question: string) => Promise<string>;
}

const defaultService: SchemaDictService = schemaDictServiceApi;

export interface ParsePreviewState {
  status: 'parsing' | 'ready' | 'error';
  schema: ParsedSchema | null;
  errorMessage: string;
}

export interface AiInterpretState {
  status: 'loading' | 'ready' | 'error';
  text: string;
  errorMessage: string;
}

export interface AiAnswerState {
  status: 'idle' | 'loading' | 'ready' | 'error';
  question: string;
  text: string;
  errorMessage: string;
}

/**
 * 数据字典 store（工厂模式，便于测试注入 fake service）。
 *
 * 维护库列表、当前库的快照与时间线、导入解析预览、两个 AI 状态。
 */
export function createSchemaDictStore(options: { service?: SchemaDictService } = {}) {
  const service = options.service ?? defaultService;

  const databases = ref<SchemaDatabase[]>([]);
  const databasesLoading = ref(false);
  const databasesError = ref('');
  const activeDatabaseId = ref('');

  const snapshots = ref<SchemaSnapshot[]>([]);
  const snapshotsLoading = ref(false);
  const activeSnapshotId = ref('');

  const timeline = ref<TimelineEntry[]>([]);
  const timelineLoading = ref(false);

  const parsePreview = ref<ParsePreviewState | null>(null);
  const importing = ref(false);

  const aiInterpret = ref<AiInterpretState | null>(null);
  const aiAnswer = ref<AiAnswerState | null>(null);

  const activeDatabase = computed(() => databases.value.find(item => item.id === activeDatabaseId.value) ?? null);

  const activeSnapshot = computed(() => snapshots.value.find(item => item.id === activeSnapshotId.value) ?? null);

  /** 最新快照（快照按时间升序，取末尾） */
  const latestSnapshot = computed(() => snapshots.value[snapshots.value.length - 1] ?? null);

  /** 当前选中快照解析出的表结构；数据损坏时返回空并记错误 */
  const activeTables = computed<ParsedTable[]>(() => {
    const snapshot = activeSnapshot.value;
    if (!snapshot?.tables_json) {
      return [];
    }
    try {
      const parsed = JSON.parse(snapshot.tables_json) as ParsedSchema;
      return parsed.tables ?? [];
    } catch {
      return [];
    }
  });

  async function loadDatabases() {
    databasesLoading.value = true;
    try {
      databases.value = await service.listSchemaDatabases();
      databasesError.value = '';
      if (!databases.value.some(item => item.id === activeDatabaseId.value)) {
        activeDatabaseId.value = databases.value[0]?.id ?? '';
        await loadDatabaseData();
      }
    } finally {
      databasesLoading.value = false;
    }
  }

  async function loadDatabaseData() {
    const databaseId = activeDatabaseId.value;
    if (!databaseId) {
      snapshots.value = [];
      timeline.value = [];
      activeSnapshotId.value = '';
      return;
    }

    snapshotsLoading.value = true;
    timelineLoading.value = true;
    try {
      const [snapshotList, timelineList] = await Promise.all([
        service.listSchemaSnapshots(databaseId),
        service.getSchemaTimeline(databaseId),
      ]);
      // 防止切换后旧数据串写
      if (activeDatabaseId.value !== databaseId) {
        return;
      }
      snapshots.value = snapshotList;
      timeline.value = timelineList;
      if (!snapshotList.some(item => item.id === activeSnapshotId.value)) {
        activeSnapshotId.value = snapshotList[snapshotList.length - 1]?.id ?? '';
      }
    } finally {
      snapshotsLoading.value = false;
      timelineLoading.value = false;
    }
  }

  function selectDatabase(databaseId: string) {
    if (activeDatabaseId.value === databaseId) {
      return;
    }
    activeDatabaseId.value = databaseId;
    activeSnapshotId.value = '';
    snapshots.value = [];
    timeline.value = [];
    aiInterpret.value = null;
    aiAnswer.value = null;
    void loadDatabaseData();
  }

  /** 新建或更新库；返回保存后的记录 */
  async function saveDatabase(item: SchemaDatabase): Promise<SchemaDatabase> {
    const saved = await service.upsertSchemaDatabase(item);
    const index = databases.value.findIndex(existing => existing.id === saved.id);
    if (index >= 0) {
      databases.value.splice(index, 1, saved);
    } else {
      databases.value.push(saved);
      databases.value.sort((a, b) => a.sort - b.sort || a.created_at - b.created_at);
      activeDatabaseId.value = saved.id;
      await loadDatabaseData();
    }
    return saved;
  }

  async function removeDatabase(databaseId: string) {
    await service.deleteSchemaDatabase(databaseId);
    databases.value = databases.value.filter(item => item.id !== databaseId);
    if (activeDatabaseId.value === databaseId) {
      activeDatabaseId.value = databases.value[0]?.id ?? '';
      activeSnapshotId.value = '';
      await loadDatabaseData();
    }
  }

  /** 干跑解析粘贴内容，生成导入预览 */
  async function previewParse(dbType: string, sourceKind: string, sourceText: string) {
    parsePreview.value = { status: 'parsing', schema: null, errorMessage: '' };
    try {
      const schema = await service.parseSchemaInput(dbType, sourceKind, sourceText);
      parsePreview.value = { status: 'ready', schema, errorMessage: '' };
    } catch (error) {
      parsePreview.value = {
        status: 'error',
        schema: null,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function clearPreview() {
    parsePreview.value = null;
  }

  /** 导入快照：落库后刷新快照与时间线并选中新快照 */
  async function importSnapshot(request: ImportSchemaSnapshotRequest): Promise<SchemaSnapshot> {
    importing.value = true;
    try {
      const saved = await service.importSchemaSnapshot(request);
      await loadDatabaseData();
      activeSnapshotId.value = saved.id;
      return saved;
    } finally {
      importing.value = false;
      parsePreview.value = null;
    }
  }

  async function removeSnapshot(snapshotId: string) {
    await service.deleteSchemaSnapshot(snapshotId);
    if (activeSnapshotId.value === snapshotId) {
      activeSnapshotId.value = '';
    }
    await loadDatabaseData();
  }

  async function reloadTimeline() {
    if (!activeDatabaseId.value) {
      return;
    }
    timelineLoading.value = true;
    try {
      timeline.value = await service.getSchemaTimeline(activeDatabaseId.value);
    } finally {
      timelineLoading.value = false;
    }
  }

  /** AI 解读变更（tab 内按钮触发，入参由视图按当前选择决定） */
  async function interpretChanges(beforeSnapshotId: string, afterSnapshotId: string) {
    aiInterpret.value = { status: 'loading', text: '', errorMessage: '' };
    try {
      const text = await service.interpretSchemaChanges(beforeSnapshotId, afterSnapshotId);
      aiInterpret.value = { status: 'ready', text, errorMessage: '' };
    } catch (error) {
      aiInterpret.value = {
        status: 'error',
        text: '',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function dismissInterpret() {
    aiInterpret.value = null;
  }

  /** AI 表结构问答（单轮） */
  async function askQuestion(question: string) {
    const snapshotId = activeSnapshotId.value;
    if (!snapshotId || aiAnswer.value?.status === 'loading') {
      return;
    }
    aiAnswer.value = { status: 'loading', question, text: '', errorMessage: '' };
    try {
      const text = await service.askSchemaDict(snapshotId, question);
      if (activeSnapshotId.value !== snapshotId) {
        return;
      }
      aiAnswer.value = { status: 'ready', question, text, errorMessage: '' };
    } catch (error) {
      if (activeSnapshotId.value !== snapshotId) {
        return;
      }
      aiAnswer.value = {
        status: 'error',
        question,
        text: '',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function clearAnswer() {
    aiAnswer.value = null;
  }

  return {
    // 状态
    databases,
    databasesLoading,
    databasesError,
    activeDatabaseId,
    snapshots,
    snapshotsLoading,
    activeSnapshotId,
    timeline,
    timelineLoading,
    parsePreview,
    importing,
    aiInterpret,
    aiAnswer,
    // 派生
    activeDatabase,
    activeSnapshot,
    latestSnapshot,
    activeTables,
    // 动作
    loadDatabases,
    selectDatabase,
    saveDatabase,
    removeDatabase,
    previewParse,
    clearPreview,
    importSnapshot,
    removeSnapshot,
    reloadTimeline,
    interpretChanges,
    dismissInterpret,
    askQuestion,
    clearAnswer,
  };
}

export type SchemaDictStore = ReturnType<typeof createSchemaDictStore>;

export const useSchemaDictStore = defineStore('schema-dict', () => createSchemaDictStore());

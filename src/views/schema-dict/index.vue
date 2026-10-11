<script setup lang="ts">
import { CloudOff, Database, FolderPlus, Loader2, Plus, RefreshCw, Settings2, Trash2 } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import SagStatsStrip from '@/components/sag/sag-stats-strip/index.vue';
import type { StatTile } from '@/components/sag/sag-stats-strip/index.vue';
import { Button } from '@/components/ui/button';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useConfirm } from '@/composables/use-confirm';
import { useSchemaDictStore } from '@/stores/schema-dict';
import type { SchemaDatabase } from '@/types/schema-dict';
import { getErrorMessage } from '@/utils/helpers';
import { formatShortDate } from '@/utils/stats';

import SchemaAiPanel from './_components/schema-ai-panel.vue';
import SchemaDatabaseDialog from './_components/schema-database-dialog.vue';
import SchemaImportDialog from './_components/schema-import-dialog.vue';
import SchemaTableDetail from './_components/schema-table-detail.vue';
import SchemaTableList from './_components/schema-table-list.vue';
import SchemaTimelinePanel from './_components/schema-timeline-panel.vue';
import { computeSchemaStats } from './utils/display';

type PageTab = 'tables' | 'timeline' | 'ai';

const store = useSchemaDictStore();
const confirm = useConfirm();

const activeTab = ref<PageTab>('tables');
const activeTableName = ref('');

const importOpen = ref(false);
const databaseDialogOpen = ref(false);
const editingDatabase = ref<SchemaDatabase | null>(null);

const hasDatabases = computed(() => store.databases.length > 0);
const hasSnapshots = computed(() => store.snapshots.length > 0);
const showDatabaseSkeleton = computed(() => store.databasesLoading && !store.databases.length);

const activeTable = computed(() => store.activeTables.find(table => table.name === activeTableName.value) ?? null);

const schemaStats = computed(() => computeSchemaStats(store.databases, store.snapshots, store.activeTables.length));
const statTiles = computed<StatTile[]>(() => [
  { label: '库', value: schemaStats.value.databaseCount },
  { label: '快照（当前库）', value: schemaStats.value.snapshotCount },
  { label: '表（当前库）', value: schemaStats.value.tableCount },
  {
    label: '最近快照',
    value: schemaStats.value.lastSnapshotAt ? formatShortDate(schemaStats.value.lastSnapshotAt) : '—',
  },
]);

onMounted(() => {
  void store.loadDatabases();
});

// ===== 库管理 =====

function openCreateDatabase() {
  editingDatabase.value = null;
  databaseDialogOpen.value = true;
}

function openEditDatabase() {
  if (!store.activeDatabase) {
    return;
  }
  editingDatabase.value = store.activeDatabase;
  databaseDialogOpen.value = true;
}

async function handleSaveDatabase(item: SchemaDatabase) {
  try {
    const saved = await store.saveDatabase(item);
    databaseDialogOpen.value = false;
    toast.success(`已保存「${saved.name}」`);
  } catch (error) {
    toast.error(getErrorMessage(error, '保存库失败'));
  }
}

async function handleDeleteDatabase(item: SchemaDatabase) {
  const proceed = await confirm.ask({
    title: '删除库',
    description: `确定删除「${item.name}」吗？它下面所有快照都会一起删除，无法恢复。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    await store.removeDatabase(item.id);
    databaseDialogOpen.value = false;
    activeTableName.value = '';
    toast.success('库已删除');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除库失败'));
  }
}

// ===== 快照导入 =====

function handlePreview(payload: { dbType: string; sourceKind: string; sourceText: string }) {
  void store.previewParse(payload.dbType, payload.sourceKind, payload.sourceText);
}

async function handleImportConfirm(payload: {
  databaseId: string;
  label: string;
  note: string;
  sourceKind: string;
  sourceText: string;
}) {
  try {
    const saved = await store.importSnapshot({
      database_id: payload.databaseId,
      label: payload.label,
      note: payload.note,
      source_kind: payload.sourceKind as 'ddl' | 'query',
      source_text: payload.sourceText,
    });
    importOpen.value = false;
    activeTableName.value = '';
    toast.success(`快照「${saved.label}」已保存`);
  } catch (error) {
    toast.error(getErrorMessage(error, '导入快照失败'));
  }
}

async function handleDeleteSnapshot() {
  const snapshot = store.activeSnapshot;
  if (!snapshot) {
    return;
  }
  const proceed = await confirm.ask({
    title: '删除快照',
    description: `确定删除快照「${snapshot.label}」吗？时间线会随之重新计算。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    await store.removeSnapshot(snapshot.id);
    toast.success('快照已删除');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除快照失败'));
  }
}

// ===== AI =====

function handleInterpret(payload: { beforeId: string; afterId: string }) {
  void store.interpretChanges(payload.beforeId, payload.afterId);
}

function handleAsk(question: string) {
  void store.askQuestion(question);
}

function retryLoad() {
  void store.loadDatabases();
}

function setDbTypeFilter(value: unknown) {
  store.selectDatabase(value as string);
  activeTableName.value = '';
}

function setSnapshotFilter(value: unknown) {
  store.activeSnapshotId = value as string;
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <SagPageHeader class="px-6 pt-6" title="数据字典">
      <template #meta>
        <span v-if="store.activeDatabase" class="text-sm text-muted-foreground">
          {{ store.activeDatabase.note || `${store.databases.length} 个库 · ${store.activeTables.length} 张表` }}
        </span>
      </template>
      <template #actions>
        <Button type="button" :disabled="!hasDatabases" @click="importOpen = true">
          <Plus class="size-4" />
          导入快照
        </Button>
      </template>
    </SagPageHeader>

    <div class="flex min-h-0 flex-1 flex-col gap-4 px-6 pb-6 pt-4">
      <!-- 加载失败 -->
      <div
        v-if="store.databasesError"
        class="flex flex-col items-center gap-3 rounded-xl border border-destructive/30 bg-destructive/5 px-6 py-10 text-center"
      >
        <CloudOff class="size-8 text-destructive" />
        <p class="max-w-md text-sm text-muted-foreground">{{ store.databasesError }}</p>
        <Button type="button" variant="outline" size="sm" @click="retryLoad">
          <RefreshCw class="size-3.5" />
          重试
        </Button>
      </div>

      <template v-else>
        <SagStatsStrip v-if="hasDatabases" class="shrink-0" :tiles="statTiles" />

        <!-- 选择器行 -->
        <div class="flex flex-wrap items-center gap-2">
          <Select v-if="hasDatabases" :model-value="store.activeDatabaseId" @update:model-value="setDbTypeFilter">
            <SelectTrigger class="h-9 w-52" aria-label="选择库">
              <SelectValue placeholder="选择库" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="item of store.databases" :key="item.id" :value="item.id">
                {{ item.name }}
              </SelectItem>
            </SelectContent>
          </Select>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-9 text-muted-foreground"
            aria-label="新建库"
            title="新建库"
            @click="openCreateDatabase"
          >
            <FolderPlus class="size-4" />
          </Button>
          <Button
            v-if="hasDatabases"
            type="button"
            variant="ghost"
            size="icon"
            class="size-9 text-muted-foreground"
            aria-label="编辑当前库"
            title="编辑当前库"
            @click="openEditDatabase"
          >
            <Settings2 class="size-4" />
          </Button>

          <template v-if="hasDatabases && hasSnapshots">
            <Select :model-value="store.activeSnapshotId" @update:model-value="setSnapshotFilter">
              <SelectTrigger class="h-9 w-56" aria-label="选择快照版本">
                <SelectValue placeholder="快照版本" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item of store.snapshots" :key="item.id" :value="item.id">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              class="size-9 text-muted-foreground"
              aria-label="删除当前快照"
              title="删除当前快照"
              @click="handleDeleteSnapshot"
            >
              <Trash2 class="size-4" />
            </Button>
          </template>
        </div>

        <!-- 加载中 -->
        <div v-if="showDatabaseSkeleton" class="space-y-3" aria-busy="true">
          <Skeleton class="h-9 w-64 rounded-lg" />
          <Skeleton class="h-64 w-full rounded-xl" />
        </div>

        <!-- 空状态：还没有库 -->
        <div v-else-if="!hasDatabases" class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center">
          <div class="flex size-12 items-center justify-center rounded-full bg-primary/10">
            <Database class="size-6 text-primary" />
          </div>
          <div class="space-y-1">
            <h2 class="text-base font-semibold">建一个数据字典</h2>
            <p class="max-w-md text-sm text-muted-foreground">
              给公司每个库建一个条目（病案、体检…），定期把表结构快照导进来，就能随时翻表结构、看每次升级改了什么。
            </p>
          </div>
          <Button type="button" variant="outline" size="sm" @click="openCreateDatabase">
            <Plus class="size-3.5" />
            新建库
          </Button>
        </div>

        <template v-else>
          <!-- 页面主体 Tabs -->
          <Tabs
            :model-value="activeTab"
            class="flex min-h-0 flex-1 flex-col gap-4"
            @update:model-value="value => (activeTab = value as PageTab)"
          >
            <TabsList class="w-fit">
              <TabsTrigger value="tables">表结构</TabsTrigger>
              <TabsTrigger value="timeline">变更时间线</TabsTrigger>
              <TabsTrigger value="ai">AI 问答</TabsTrigger>
            </TabsList>

            <!-- 无快照的空状态（表结构/时间线共用） -->
            <div
              v-if="!hasSnapshots && activeTab !== 'ai'"
              class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center"
            >
              <div class="flex size-12 items-center justify-center rounded-full bg-primary/10">
                <Plus class="size-6 text-primary" />
              </div>
              <div class="space-y-1">
                <h2 class="text-base font-semibold">还没有快照</h2>
                <p class="max-w-md text-sm text-muted-foreground">
                  点右上角「导入快照」：贴一份建表 DDL，或在 SQL 客户端里跑一条我们提供的查询把结果粘回来。
                </p>
              </div>
            </div>

            <!-- 表结构 -->
            <div v-else-if="activeTab === 'tables'" class="min-h-0 flex-1">
              <div
                v-if="store.snapshotsLoading"
                class="flex h-full items-center justify-center text-sm text-muted-foreground"
                aria-busy="true"
              >
                <Loader2 class="mr-2 size-4 animate-spin" />
                正在加载快照…
              </div>
              <ResizablePanelGroup v-else direction="horizontal" class="min-h-0 flex-1 rounded-xl">
                <ResizablePanel id="schema-tables-panel" :default-size="28" :min-size="18">
                  <SchemaTableList
                    :tables="store.activeTables"
                    :active-name="activeTableName"
                    @select="table => (activeTableName = table.name)"
                  />
                </ResizablePanel>
                <ResizableHandle id="schema-detail-handle" with-handle />
                <ResizablePanel id="schema-detail-panel" class="min-h-0">
                  <div class="h-full overflow-y-auto pl-4">
                    <SchemaTableDetail :table="activeTable" />
                  </div>
                </ResizablePanel>
              </ResizablePanelGroup>
            </div>

            <!-- 变更时间线 -->
            <div v-else-if="activeTab === 'timeline'" class="min-h-0 flex-1 overflow-y-auto pr-1">
              <SchemaTimelinePanel
                :entries="store.timeline"
                :loading="store.timelineLoading"
                :interpret="store.aiInterpret"
                @interpret="handleInterpret"
                @dismiss-interpret="store.dismissInterpret()"
              />
            </div>

            <!-- AI 问答 -->
            <div v-else class="flex min-h-0 flex-1 flex-col">
              <SchemaAiPanel
                :answer="store.aiAnswer"
                :disabled="!hasSnapshots"
                @ask="handleAsk"
                @clear="store.clearAnswer()"
              />
            </div>
          </Tabs>
        </template>
      </template>
    </div>

    <SchemaImportDialog
      :open="importOpen"
      :databases="store.databases"
      :active-database-id="store.activeDatabaseId"
      :preview="store.parsePreview"
      :importing="store.importing"
      @update:open="
        value => {
          importOpen = value;
          if (value) {
            store.clearPreview();
          }
        }
      "
      @preview="handlePreview"
      @confirm="handleImportConfirm"
    />

    <SchemaDatabaseDialog
      :open="databaseDialogOpen"
      :database="editingDatabase"
      @update:open="value => (databaseDialogOpen = value)"
      @save="handleSaveDatabase"
      @delete="handleDeleteDatabase"
    />
  </div>
</template>

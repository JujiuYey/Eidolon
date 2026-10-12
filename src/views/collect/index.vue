<script setup lang="ts">
import { ArchiveX, Bookmark, CloudOff, RefreshCw, Search, Sparkles, Trash2 } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import ConfirmDialog from '@/components/ConfirmDialog.vue';
import SagBarChart from '@/components/sag/sag-bar-chart/index.vue';
import SagDonutChart from '@/components/sag/sag-donut-chart/index.vue';
import type { DonutSegment } from '@/components/sag/sag-donut-chart/index.vue';
import SagHBarList from '@/components/sag/sag-h-bar-list/index.vue';
import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import SagStatsStrip from '@/components/sag/sag-stats-strip/index.vue';
import type { StatTile } from '@/components/sag/sag-stats-strip/index.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useConfirm } from '@/composables/use-confirm';
import { useCollectStore } from '@/stores/collect';
import type { CollectItem } from '@/types/collect';
import { getErrorMessage } from '@/utils/helpers';
import { dailyLabels } from '@/utils/stats';

import CollectAiPanel from './_components/collect-ai-panel.vue';
import CollectItemEditorDialog from './_components/collect-item-editor-dialog.vue';
import CollectItemList from './_components/collect-item-list.vue';
import CollectQuickAdd from './_components/collect-quick-add.vue';
import CollectTrashPanel from './_components/collect-trash-panel.vue';
import type { CollectFilter } from './utils/display';
import { computeCollectStats, filterCounts, filterItems } from './utils/display';

const store = useCollectStore();
const confirm = useConfirm();

const filter = ref<CollectFilter>('all');
const query = ref('');

const editorOpen = ref(false);
const editingItem = ref<CollectItem | null>(null);
const aiPanelOpen = ref(false);

const counts = computed(() => filterCounts(store.items));
const visibleItems = computed(() => filterItems(store.items, filter.value, query.value));
const hasAnyItem = computed(() => store.items.length > 0);
const showLoadingSkeleton = computed(() => store.itemsLoading && !store.itemsLoaded);

const collectStats = computed(() => computeCollectStats(store.items));
const statTiles = computed<StatTile[]>(() => [
  { label: '本周新增', value: collectStats.value.weekCount },
  { label: '30 天新增', value: collectStats.value.monthCount },
  { label: '连续收集', value: collectStats.value.streakDays, hint: '天' },
  { label: '已归档', value: collectStats.value.archivedCount },
]);
const trendValues = computed(() => collectStats.value.dailyCounts);
const trendLabels = computed(() => dailyLabels(trendValues.value.length));

const kindSegments = computed<DonutSegment[]>(() => [
  { label: '链接', value: counts.value.link, tone: 'sky' },
  { label: '记事', value: counts.value.note, tone: 'amber' },
]);

const hostBarItems = computed(() =>
  collectStats.value.topHosts.map(entry => ({ label: entry.host, value: entry.count })),
);
onMounted(() => {
  void store.loadItems();
});

async function handleQuickSubmit(text: string) {
  try {
    await store.createFromText(text);
    toast.success('已收藏');
  } catch (error) {
    toast.error(getErrorMessage(error, '收藏失败'));
  }
}

function handleHostClick(host: string) {
  query.value = host;
  filter.value = 'all';
}

function openEdit(item: CollectItem) {
  editingItem.value = item;
  editorOpen.value = true;
}

/** 链接卡点击：在系统默认浏览器打开 */
async function handleOpen(item: CollectItem) {
  if (!item.url) {
    return;
  }
  try {
    await store.openInBrowser(item.url);
  } catch (error) {
    toast.error(getErrorMessage(error, '打开链接失败'));
  }
}

async function handleSave(item: CollectItem) {
  try {
    await store.saveItem(item);
    editorOpen.value = false;
    toast.success('已保存');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

async function handleCopy(item: CollectItem) {
  if (!item.url) {
    return;
  }
  try {
    await navigator.clipboard.writeText(item.url);
    toast.success('链接已复制');
  } catch (error) {
    toast.error(getErrorMessage(error, '复制失败'));
  }
}

function handleRefresh(item: CollectItem) {
  void store.refreshLink(item);
  toast.success('正在重新获取…');
}

function handleSummarize(item: CollectItem) {
  void store.summarizeLink(item);
}

function handleAsk(question: string) {
  void store.askCollection(question);
}

function handleSummarizeDismiss(item: CollectItem) {
  store.dismissSummary(item.id);
}

async function handleArchive(item: CollectItem) {
  try {
    const saved = await store.toggleArchived(item);
    toast.success(saved.archived ? '已归档' : '已取消归档');
  } catch (error) {
    toast.error(getErrorMessage(error, '归档失败'));
  }
}

async function handleRemove(item: CollectItem) {
  const name = item.kind === 'link' ? (item.title ?? item.url) : item.content;
  const proceed = await confirm.ask({
    title: '删除收藏',
    description: `确定删除「${name}」吗？删除后会在回收站保留 30 天，可随时恢复。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    await store.removeItem(item.id);
    editorOpen.value = false;
    toast.success('已移入回收站');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

// ===== 回收站 =====

const trashMode = ref(false);

function toggleTrash() {
  trashMode.value = !trashMode.value;
  if (trashMode.value) {
    void store.loadTrash();
  } else {
    void store.loadItems();
  }
}

async function handleRestore(item: CollectItem) {
  try {
    await store.restoreItem(item);
    toast.success('已恢复');
  } catch (error) {
    toast.error(getErrorMessage(error, '恢复失败'));
  }
}

async function handlePurge(item: CollectItem) {
  const name = item.kind === 'link' ? (item.title ?? item.url) : item.content;
  const proceed = await confirm.ask({
    title: '彻底删除',
    description: `确定彻底删除「${name}」吗？此操作无法恢复。`,
    confirmLabel: '彻底删除',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    await store.purgeItem(item.id);
    toast.success('已彻底删除');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

async function handleEmptyTrash() {
  const count = store.trashItems.length;
  const proceed = await confirm.ask({
    title: '清空回收站',
    description: `确定清空回收站里的 ${count} 条收藏吗？此操作无法恢复。`,
    confirmLabel: '清空',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    const purged = await store.emptyTrash();
    toast.success(`已清除 ${purged} 条`);
  } catch (error) {
    toast.error(getErrorMessage(error, '清空失败'));
  }
}

function retryLoad() {
  void store.loadItems();
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <SagPageHeader class="px-6 pt-6" title="收藏夹">
      <template #meta>
        <span class="text-sm text-muted-foreground"
          >共 {{ counts.all }} 条<template v-if="counts.archived > 0"> · 归档 {{ counts.archived }}</template></span
        >
      </template>
      <template #actions>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          :class="trashMode ? 'bg-muted text-foreground' : 'text-muted-foreground'"
          aria-label="回收站"
          title="回收站"
          @click="toggleTrash"
        >
          <Trash2 class="size-4" />
        </Button>
        <Button type="button" variant="outline" @click="aiPanelOpen = !aiPanelOpen">
          <Sparkles class="size-4" />
          问收藏
        </Button>
      </template>
    </SagPageHeader>

    <div class="flex flex-1 flex-col gap-4 px-6 pb-6 pt-4">
      <!-- 回收站视图 -->
      <template v-if="trashMode">
        <CollectTrashPanel
          :items="store.trashItems"
          :loading="store.trashLoading"
          @restore="handleRestore"
          @purge="handlePurge"
          @empty="handleEmptyTrash"
        />
      </template>

      <template v-else>
        <!-- 加载失败 -->
        <div
          v-if="store.itemsError"
          class="flex flex-col items-center gap-3 rounded-xl border border-destructive/30 bg-destructive/5 px-6 py-10 text-center"
        >
          <CloudOff class="size-8 text-destructive" />
          <p class="max-w-md text-sm text-muted-foreground">
            {{ store.itemsError }}
          </p>
          <Button type="button" variant="outline" size="sm" @click="retryLoad">
            <RefreshCw class="size-3.5" />
            重试
          </Button>
        </div>

        <template v-else>
          <div v-if="hasAnyItem" class="shrink-0 space-y-4">
            <SagStatsStrip :tiles="statTiles" />

            <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <div class="rounded-xl border bg-card p-4 md:col-span-2">
                <div class="flex items-center justify-between gap-2">
                  <h3 class="text-sm font-medium">近 30 天收藏</h3>
                  <span class="text-xs text-muted-foreground">按天</span>
                </div>
                <div class="mt-3">
                  <SagBarChart
                    :values="trendValues"
                    :labels="trendLabels"
                    :tooltips="collectStats.dailyTooltips"
                    aria-label="近 30 天收藏柱状图"
                  />
                </div>
              </div>

              <div class="rounded-xl border bg-card p-4">
                <h3 class="text-sm font-medium">类型分布</h3>
                <div class="mt-3">
                  <SagDonutChart :segments="kindSegments" center-label="条收藏" aria-label="收藏类型分布环形图" />
                </div>
              </div>

              <div class="flex flex-col rounded-xl border bg-card p-4">
                <div class="flex items-center justify-between gap-2">
                  <h3 class="text-sm font-medium">常收藏站点</h3>
                  <span class="text-xs text-muted-foreground">点击按站点搜索</span>
                </div>
                <div class="mt-3 min-h-0 flex-1">
                  <SagHBarList
                    :items="hostBarItems"
                    :limit="6"
                    selectable
                    @select="item => handleHostClick(item.label)"
                  />
                </div>
              </div>
            </div>
          </div>

          <CollectQuickAdd @submit="handleQuickSubmit" />

          <CollectAiPanel
            v-if="aiPanelOpen"
            :answer="store.aiAnswer"
            :disabled="!hasAnyItem"
            @ask="handleAsk"
            @clear="store.dismissAnswer()"
            @close="aiPanelOpen = false"
          />

          <!-- 过滤 + 搜索 -->
          <div class="flex flex-wrap items-center gap-3">
            <Tabs :model-value="filter" @update:model-value="value => (filter = value as CollectFilter)">
              <TabsList>
                <TabsTrigger value="all">全部 {{ counts.all }}</TabsTrigger>
                <TabsTrigger value="link">链接 {{ counts.link }}</TabsTrigger>
                <TabsTrigger value="note">记事 {{ counts.note }}</TabsTrigger>
                <TabsTrigger value="archived">归档 {{ counts.archived }}</TabsTrigger>
              </TabsList>
            </Tabs>
            <div class="relative ml-auto w-56">
              <Search
                class="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
              />
              <Input v-model="query" class="h-9 pl-8" placeholder="搜索收藏" aria-label="搜索收藏" />
            </div>
          </div>

          <!-- 加载中 -->
          <div v-if="showLoadingSkeleton" class="space-y-2" aria-busy="true">
            <Skeleton v-for="index in 3" :key="index" class="h-16 w-full rounded-lg" />
          </div>

          <!-- 列表 -->
          <CollectItemList
            v-else-if="visibleItems.length > 0"
            :items="visibleItems"
            :summaries="store.summaries"
            @open="handleOpen"
            @edit="openEdit"
            @copy="handleCopy"
            @refresh="handleRefresh"
            @summarize="handleSummarize"
            @dismiss-summary="handleSummarizeDismiss"
            @archive="handleArchive"
            @remove="handleRemove"
          />

          <!-- 空状态 -->
          <div v-else-if="!hasAnyItem" class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center">
            <div class="flex size-12 items-center justify-center rounded-full bg-primary/10">
              <Bookmark class="size-6 text-primary" />
            </div>
            <div class="space-y-1">
              <h2 class="text-base font-semibold">收藏夹还是空的</h2>
              <p class="max-w-sm text-sm text-muted-foreground">
                在上方粘贴一个链接或随手记点什么，回车就能收进来；不用想分类，先存下来再说。
              </p>
            </div>
          </div>
          <p v-else-if="query.trim()" class="py-10 text-center text-sm text-muted-foreground">没有匹配的收藏</p>
          <div v-else class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center">
            <ArchiveX class="size-8 text-muted-foreground/50" />
            <p class="text-sm text-muted-foreground">该状态下没有收藏</p>
          </div>
        </template>
      </template>
    </div>

    <CollectItemEditorDialog v-model:open="editorOpen" :item="editingItem" @save="handleSave" />

    <ConfirmDialog
      :open="confirm.state.open"
      @cancel="confirm.onCancel"
      @confirm="confirm.onConfirm"
      @update:open="confirm.onOpenChange"
    />
  </div>
</template>

<script setup lang="ts">
import { CloudOff, Plus, RefreshCw, Sparkles } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import SagBarChart from '@/components/sag/sag-bar-chart/index.vue';
import SagDonutChart from '@/components/sag/sag-donut-chart/index.vue';
import type { DonutSegment } from '@/components/sag/sag-donut-chart/index.vue';
import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import SagStatsStrip from '@/components/sag/sag-stats-strip/index.vue';
import type { StatTile } from '@/components/sag/sag-stats-strip/index.vue';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useConfirm } from '@/composables/use-confirm';
import { useTasksStore } from '@/stores/tasks';
import type { Task, TaskDraft } from '@/types/tasks';
import { getErrorMessage } from '@/utils/helpers';
import { dailyLabels } from '@/utils/stats';

import TaskEditorDialog from './_components/task-editor-dialog.vue';
import TaskList from './_components/task-list.vue';
import TaskPlanPanel from './_components/task-plan-panel.vue';
import TaskQuickAdd from './_components/task-quick-add.vue';
import type { TaskFilter } from './utils/display';
import { computeTaskStats, filterCounts, filterTasks, sortTasks } from './utils/display';

const store = useTasksStore();
const confirm = useConfirm();

const filter = ref<TaskFilter>('all');

const editorOpen = ref(false);
const editingTask = ref<Task | null>(null);

const counts = computed(() => filterCounts(store.tasks));
const visibleTasks = computed(() => sortTasks(filterTasks(store.tasks, filter.value)));
const hasAnyTask = computed(() => store.tasks.length > 0);
const showLoadingSkeleton = computed(() => store.tasksLoading && !store.tasksLoaded);

const taskStats = computed(() => computeTaskStats(store.tasks));
const statTiles = computed<StatTile[]>(() => [
  {
    label: '逾期',
    value: taskStats.value.overdueCount,
    tone: taskStats.value.overdueCount > 0 ? 'danger' : 'default',
  },
  { label: '本周完成', value: taskStats.value.weekCompletedCount },
  { label: '30 天完成', value: taskStats.value.monthCompletedCount },
  { label: '累计完成', value: taskStats.value.totalCompletedCount },
]);
const completionValues = computed(() => taskStats.value.dailyCompleted);
const completionLabels = computed(() => dailyLabels(completionValues.value.length));

const statusSegments = computed<DonutSegment[]>(() => [
  { label: '待办', value: counts.value.todo, tone: 'slate' },
  { label: '进行中', value: counts.value.doing, tone: 'sky' },
  { label: '已完成', value: counts.value.done, tone: 'emerald' },
]);
onMounted(() => {
  void store.loadTasks();
});

async function handleQuickSubmit(text: string) {
  try {
    await store.createFromText(text);
    toast.success('任务已添加');
  } catch (error) {
    toast.error(getErrorMessage(error, '添加任务失败'));
  }
}

function handleParse(text: string) {
  void store.parseQuickAdd(text);
}

async function handleApplyDraft(draft: TaskDraft) {
  try {
    await store.applyDraft(draft);
    toast.success('任务已添加');
  } catch (error) {
    toast.error(getErrorMessage(error, '添加任务失败'));
  }
}

function openCreate() {
  editingTask.value = null;
  editorOpen.value = true;
}

function openEdit(task: Task) {
  editingTask.value = task;
  editorOpen.value = true;
}

async function handleSave(task: Task) {
  try {
    await store.saveTask(task);
    editorOpen.value = false;
    toast.success('任务已保存');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存任务失败'));
  }
}

async function handleToggle(task: Task) {
  try {
    await store.toggleDone(task);
  } catch (error) {
    toast.error(getErrorMessage(error, '更新任务失败'));
  }
}

async function handleRemove(task: Task) {
  const proceed = await confirm.ask({
    title: '删除任务',
    description: `确定删除「${task.title}」吗？删除后无法恢复。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!proceed) {
    return;
  }
  try {
    await store.removeTask(task.id);
    editorOpen.value = false;
    toast.success('任务已删除');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除任务失败'));
  }
}

function handlePlan() {
  void store.loadPlan();
}

function retryLoad() {
  void store.loadTasks();
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <SagPageHeader class="shrink-0 px-6 pt-6" title="任务">
      <template #meta>
        <span class="text-sm text-muted-foreground"
          >{{ counts.all }} 个任务 · {{ counts.todo + counts.doing }} 个未完成</span
        >
      </template>
      <template #actions>
        <Button
          type="button"
          variant="outline"
          :disabled="counts.todo + counts.doing === 0 || store.aiPlan?.status === 'loading'"
          @click="handlePlan"
        >
          <Sparkles class="size-4" />
          帮我排一排
        </Button>
        <Button type="button" @click="openCreate">
          <Plus class="size-4" />
          新建任务
        </Button>
      </template>
    </SagPageHeader>

    <div class="flex min-h-0 flex-1 flex-col gap-4 px-6 pb-6 pt-4">
      <!-- 加载失败 -->
      <div
        v-if="store.tasksError"
        class="flex flex-col items-center gap-3 rounded-xl border border-destructive/30 bg-destructive/5 px-6 py-10 text-center"
      >
        <CloudOff class="size-8 text-destructive" />
        <p class="max-w-md text-sm text-muted-foreground">
          {{ store.tasksError }}
        </p>
        <Button type="button" variant="outline" size="sm" @click="retryLoad">
          <RefreshCw class="size-3.5" />
          重试
        </Button>
      </div>

      <template v-else>
        <div v-if="hasAnyTask" class="shrink-0 space-y-4">
          <SagStatsStrip :tiles="statTiles" />

          <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            <div class="rounded-xl border bg-card p-4 md:col-span-2">
              <div class="flex items-center justify-between gap-2">
                <h3 class="text-sm font-medium">近 30 天完成</h3>
                <span class="text-xs text-muted-foreground">按天</span>
              </div>
              <div class="mt-3">
                <SagBarChart
                  :values="completionValues"
                  :labels="completionLabels"
                  :tooltips="taskStats.dailyTooltips"
                  aria-label="近 30 天完成任务柱状图"
                />
              </div>
            </div>

            <div class="rounded-xl border bg-card p-4">
              <h3 class="text-sm font-medium">任务状态分布</h3>
              <div class="mt-3">
                <SagDonutChart :segments="statusSegments" center-label="个任务" aria-label="任务状态分布环形图" />
              </div>
            </div>
          </div>
        </div>

        <TaskQuickAdd
          class="shrink-0"
          :draft="store.aiDraft"
          @submit="handleQuickSubmit"
          @parse="handleParse"
          @apply="handleApplyDraft"
          @discard="store.discardDraft()"
        />

        <TaskPlanPanel
          v-if="store.aiPlan"
          class="shrink-0"
          :plan="store.aiPlan"
          @dismiss="store.dismissPlan()"
          @retry="handlePlan"
        />

        <!-- 过滤 -->
        <Tabs class="shrink-0" :model-value="filter" @update:model-value="value => (filter = value as TaskFilter)">
          <TabsList>
            <TabsTrigger value="all">全部 {{ counts.all }}</TabsTrigger>
            <TabsTrigger value="todo">待办 {{ counts.todo }}</TabsTrigger>
            <TabsTrigger value="doing">进行中 {{ counts.doing }}</TabsTrigger>
            <TabsTrigger value="done">已完成 {{ counts.done }}</TabsTrigger>
          </TabsList>
        </Tabs>

        <!-- 列表区：唯一滚动区域 -->
        <div class="flex min-h-0 flex-1 flex-col overflow-y-auto">
          <!-- 加载中 -->
          <div v-if="showLoadingSkeleton" class="space-y-2" aria-busy="true">
            <Skeleton v-for="index in 3" :key="index" class="h-14 w-full rounded-lg" />
          </div>

          <!-- 列表 -->
          <TaskList
            v-else-if="visibleTasks.length > 0"
            :tasks="visibleTasks"
            @toggle="handleToggle"
            @edit="openEdit"
            @remove="handleRemove"
          />

          <!-- 空状态 -->
          <div v-else-if="!hasAnyTask" class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-center">
            <div class="flex size-12 items-center justify-center rounded-full bg-primary/10">
              <Sparkles class="size-6 text-primary" />
            </div>
            <div class="space-y-1">
              <h2 class="text-base font-semibold">还没有任务</h2>
              <p class="max-w-sm text-sm text-muted-foreground">
                在上方输入一句话直接回车记下要做的事；也可以点 ✦ 让 AI 把它整理成带优先级和截止日期的任务。
              </p>
            </div>
            <Button type="button" variant="outline" size="sm" @click="openCreate">
              <Plus class="size-3.5" />
              新建任务
            </Button>
          </div>
          <p v-else class="py-10 text-center text-sm text-muted-foreground">该状态下没有任务</p>
        </div>
      </template>
    </div>

    <TaskEditorDialog v-model:open="editorOpen" :task="editingTask" @save="handleSave" @delete="handleRemove" />
  </div>
</template>

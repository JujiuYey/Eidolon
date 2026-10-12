<script setup lang="ts">
import { FolderInput, Loader2, Network, Pencil, Plus, Trash2 } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import ConfirmDialog from '@/components/ConfirmDialog.vue';
import SagBarChart from '@/components/sag/sag-bar-chart/index.vue';
import SagDonutChart from '@/components/sag/sag-donut-chart/index.vue';
import type { DonutSegment, DonutTone } from '@/components/sag/sag-donut-chart/index.vue';
import SagHBarList from '@/components/sag/sag-h-bar-list/index.vue';
import SagStatsStrip from '@/components/sag/sag-stats-strip/index.vue';
import type { StatTile } from '@/components/sag/sag-stats-strip/index.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import { useConfirm } from '@/composables/use-confirm';
import { getApiClientStats } from '@/services';
import { useApiClientStore } from '@/stores/api-client';
import type { ApiClientOverviewStats } from '@/types/api-client';
import { dailyCounts, dailyLabels, dailyTooltips } from '@/utils/stats';

import CodeImportDialog from './components/CodeImportDialog.vue';
import ProjectFormDialog from './components/ProjectFormDialog.vue';

const router = useRouter();
const store = useApiClientStore();
const confirm = useConfirm();

const createDialogOpen = ref(false);
const editDialogOpen = ref(false);
const editingProjectId = ref<string | null>(null);
const backendError = ref<string | null>(null);
const importDialogProjectId = ref<string | null>(null);

const sortedProjects = computed(() => store.sortedProjects);

const stats = ref<ApiClientOverviewStats | null>(null);

/** 方法语义色（沿用工作区 request-helpers 的方法配色惯例） */
const METHOD_TONES: Record<string, DonutTone> = {
  GET: 'emerald',
  POST: 'amber',
  PUT: 'sky',
  PATCH: 'teal',
  DELETE: 'red',
  HEAD: 'violet',
};

const DAY_COUNT = 30;

const statTiles = computed<StatTile[]>(() => {
  const executions = stats.value?.recentExecutions;
  const successRate =
    executions && executions.total > 0 ? Math.round((executions.success / executions.total) * 100) : null;
  return [
    { label: '项目', value: stats.value?.projectCount ?? sortedProjects.value.length },
    { label: '请求', value: stats.value?.requestCount ?? 0 },
    { label: '分组', value: stats.value?.groupCount ?? 0 },
    { label: '环境', value: stats.value?.environmentCount ?? 0 },
    {
      label: '近期执行成功率',
      value: successRate === null ? '—' : `${successRate}%`,
      hint:
        executions && executions.total > 0
          ? `${executions.total} 次 · 均耗 ${executions.avgDurationMs}ms`
          : '暂无执行记录',
      tone: successRate !== null && successRate < 90 ? 'danger' : 'default',
    },
  ];
});

const createdValues = computed(() => dailyCounts(stats.value?.recentRequestTimestamps ?? [], DAY_COUNT));
const createdTooltips = computed(() => dailyTooltips(createdValues.value, '个新增'));
const createdLabels = computed(() => dailyLabels(DAY_COUNT));

const executionValues = computed(() =>
  dailyCounts(stats.value?.recentExecutions.executedAtTimestamps ?? [], DAY_COUNT),
);
const executionTooltips = computed(() => dailyTooltips(executionValues.value, '次执行'));
const executionHint = computed(() => {
  const executions = stats.value?.recentExecutions;
  if (!executions || executions.total === 0) {
    return '窗口内没有执行记录';
  }
  const rate = Math.round((executions.success / executions.total) * 100);
  return `成功率 ${rate}% · 平均耗时 ${executions.avgDurationMs}ms`;
});

const methodSegments = computed<DonutSegment[]>(() =>
  (stats.value?.methodCounts ?? []).map(item => ({
    label: item.method,
    value: item.count,
    tone: METHOD_TONES[item.method] ?? 'slate',
  })),
);

const projectBarItems = computed(() =>
  (stats.value?.projectRequestCounts ?? []).map(item => ({ label: item.projectName, value: item.count })),
);

/** 统计失败不影响主功能：静默置空，页面已有错误提示通道 */
async function loadStats(): Promise<void> {
  try {
    stats.value = await getApiClientStats();
  } catch {
    stats.value = null;
  }
}

const importDialogProject = computed(() => {
  if (!importDialogProjectId.value) {
    return null;
  }
  return store.projects.find(item => item.id === importDialogProjectId.value) ?? null;
});

function openImportDialog(projectId: string): void {
  importDialogProjectId.value = projectId;
}

async function bootstrap(): Promise<void> {
  backendError.value = null;
  try {
    await store.loadProjects();
  } catch (error) {
    console.error(error);
    backendError.value = error instanceof Error ? error.message : '加载项目失败';
  }
  void loadStats();
}

async function handleCreateProjectSubmit(payload: { name: string; description: string }): Promise<void> {
  try {
    const project = await store.createProject(payload.name, payload.description);
    createDialogOpen.value = false;
    toast.success(`已创建项目 ${project.name}`);
    await router.push({ name: 'api-client-project', params: { id: project.id } });
  } catch (error) {
    const message = error instanceof Error ? error.message : '创建项目失败';
    toast.error(message);
  }
}

function openCreateProjectDialog(): void {
  createDialogOpen.value = true;
}

function openEditProjectDialog(projectId: string): void {
  const project = store.projects.find(item => item.id === projectId);
  if (!project) {
    return;
  }
  editingProjectId.value = projectId;
  editDialogOpen.value = true;
}

async function handleEditProjectSubmit(payload: { name: string; description: string }): Promise<void> {
  if (!editingProjectId.value) {
    return;
  }
  const id = editingProjectId.value;
  try {
    await store.updateProject(id, payload.name, payload.description);
    editDialogOpen.value = false;
    toast.success('项目已更新');
    void loadStats();
  } catch (error) {
    const message = error instanceof Error ? error.message : '更新项目失败';
    toast.error(message);
  }
}

const editingProject = computed(() => {
  if (!editingProjectId.value) {
    return null;
  }
  return store.projects.find(item => item.id === editingProjectId.value) ?? null;
});

async function handleDeleteProject(projectId: string): Promise<void> {
  const project = store.projects.find(item => item.id === projectId);
  const ok = await confirm.ask({
    title: '删除项目',
    description: `确定删除项目「${project?.name ?? ''}」？此操作将一并删除该项目下的所有请求、分组和环境。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  try {
    await store.deleteProject(projectId);
    toast.success('项目已删除');
    void loadStats();
  } catch (error) {
    const message = error instanceof Error ? error.message : '删除项目失败';
    toast.error(message);
  }
}

function handleOpen(projectId: string): void {
  void router.push({ name: 'api-client-project', params: { id: projectId } });
}

function formatTimestamp(timestamp: number): string {
  if (!timestamp) {
    return '—';
  }
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return '—';
  }
  return date.toLocaleString('zh-CN');
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (store.isDirty) {
    event.preventDefault();
    event.returnValue = '';
  }
}

onMounted(() => {
  void bootstrap();
  window.addEventListener('beforeunload', handleBeforeUnload);
});

onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', handleBeforeUnload);
});
</script>

<template>
  <div class="w-full h-full flex flex-col overflow-hidden p-6">
    <div class="mb-6 shrink-0 flex flex-col gap-4 border-b pb-5 lg:flex-row lg:items-end lg:justify-between">
      <div>
        <h1 class="flex items-center gap-2 text-2xl font-semibold tracking-tight text-foreground">
          <Network class="size-6 text-primary" />
          API 管理
        </h1>
        <p class="mt-2 text-sm text-muted-foreground">
          按项目管理 API 请求、环境变量与执行历史。点击项目卡片进入工作区。
        </p>
      </div>

      <Button @click="openCreateProjectDialog">
        <Plus class="size-4" />
        新建项目
      </Button>
    </div>

    <Alert v-if="backendError" variant="destructive" class="mb-4">
      <AlertTitle>无法连接到接口请求后端</AlertTitle>
      <AlertDescription> {{ backendError }}。请确认 Rust 端 API Client commands 已注册后重启应用。 </AlertDescription>
    </Alert>

    <div v-if="stats && !store.isLoadingProjects && sortedProjects.length > 0" class="mb-4 shrink-0 space-y-4">
      <SagStatsStrip :tiles="statTiles" />

      <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <div class="rounded-xl border bg-card p-4 md:col-span-2">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-medium">近 30 天新增请求</h3>
            <span class="text-xs text-muted-foreground">按天</span>
          </div>
          <div class="mt-3">
            <SagBarChart
              :values="createdValues"
              :labels="createdLabels"
              :tooltips="createdTooltips"
              aria-label="近 30 天新增请求柱状图"
            />
          </div>
        </div>

        <div class="rounded-xl border bg-card p-4">
          <h3 class="text-sm font-medium">请求方法分布</h3>
          <div class="mt-3">
            <SagDonutChart :segments="methodSegments" center-label="个请求" aria-label="请求方法分布环形图" />
          </div>
        </div>

        <div class="flex flex-col rounded-xl border bg-card p-4">
          <h3 class="text-sm font-medium">各项目请求数</h3>
          <div class="mt-3 min-h-0 flex-1">
            <SagHBarList :items="projectBarItems" />
          </div>
        </div>

        <div class="rounded-xl border bg-card p-4 md:col-span-2">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-medium">近 30 天执行分布</h3>
            <span class="text-xs text-muted-foreground">{{ executionHint }}</span>
          </div>
          <div class="mt-3">
            <SagBarChart
              :values="executionValues"
              :labels="createdLabels"
              :tooltips="executionTooltips"
              aria-label="近 30 天执行分布柱状图"
            />
          </div>
        </div>
      </div>
    </div>

    <div v-if="store.isLoadingProjects" class="flex flex-1 items-center justify-center">
      <div class="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在加载项目…
      </div>
    </div>

    <div v-else-if="sortedProjects.length === 0" class="flex flex-1 items-center justify-center">
      <Card class="w-full max-w-xl border-dashed bg-muted/10">
        <CardHeader class="items-center text-center">
          <div class="mx-auto flex size-14 items-center justify-center rounded-full border bg-background">
            <Network class="size-6 text-primary" />
          </div>
          <CardTitle>还没有 API 项目</CardTitle>
          <CardDescription> 在上方输入项目名称并点击「新建项目」开始。 </CardDescription>
        </CardHeader>
      </Card>
    </div>

    <div v-else class="grid flex-1 content-start gap-4 overflow-y-auto pb-4 md:grid-cols-2 xl:grid-cols-3">
      <div
        v-for="project of sortedProjects"
        :key="project.id"
        class="group flex flex-col rounded-2xl border bg-card p-5 text-left shadow-sm transition-colors hover:border-primary/30 hover:bg-primary/5"
      >
        <div class="flex items-start justify-between gap-3">
          <button type="button" class="flex min-w-0 flex-1 items-start gap-3 text-left" @click="handleOpen(project.id)">
            <div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
              <Network class="size-5" />
            </div>
            <div class="min-w-0">
              <h2 class="truncate text-lg font-semibold tracking-tight text-foreground">
                {{ project.name }}
              </h2>
              <p class="mt-1 line-clamp-2 text-sm leading-6 text-muted-foreground">
                {{ project.description || '这个项目还没有填写描述。' }}
              </p>
            </div>
          </button>
          <div class="flex shrink-0 items-center gap-2">
            <Badge variant="outline" class="border-border bg-muted text-muted-foreground"> HTTP </Badge>
            <Button
              variant="ghost"
              size="icon-sm"
              :aria-label="`从代码导入 ${project.name}`"
              @click="openImportDialog(project.id)"
            >
              <FolderInput class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              :disabled="store.isDeletingProject === project.id"
              :aria-label="`编辑项目 ${project.name}`"
              @click="openEditProjectDialog(project.id)"
            >
              <Pencil class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              :disabled="store.isDeletingProject === project.id"
              :aria-label="`删除项目 ${project.name}`"
              @click="handleDeleteProject(project.id)"
            >
              <Trash2 class="size-4" />
            </Button>
          </div>
        </div>

        <div class="mt-4 flex items-center justify-between text-xs text-muted-foreground">
          <span>创建于 {{ formatTimestamp(project.createdAt) }}</span>
          <span>更新于 {{ formatTimestamp(project.updatedAt) }}</span>
        </div>
      </div>
    </div>

    <Spinner v-if="store.isLoadingProjects" class="pointer-events-none fixed right-6 top-6" />

    <ConfirmDialog
      :open="confirm.state.open"
      :title="confirm.state.title"
      :description="confirm.state.description"
      :confirm-label="confirm.state.confirmLabel"
      :cancel-label="confirm.state.cancelLabel"
      :destructive="confirm.state.destructive"
      @update:open="confirm.onOpenChange"
      @confirm="confirm.onConfirm"
      @cancel="confirm.onCancel"
    />

    <ProjectFormDialog
      mode="create"
      :open="createDialogOpen"
      @update:open="(value: boolean) => (createDialogOpen = value)"
      @submit="handleCreateProjectSubmit"
    />

    <ProjectFormDialog
      mode="edit"
      :open="editDialogOpen"
      :initial-name="editingProject?.name"
      :initial-description="editingProject?.description"
      @update:open="(value: boolean) => (editDialogOpen = value)"
      @submit="handleEditProjectSubmit"
    />

    <CodeImportDialog
      :open="importDialogProjectId !== null"
      :project="importDialogProject"
      @update:open="(value: boolean) => !value && (importDialogProjectId = null)"
    />
  </div>
</template>

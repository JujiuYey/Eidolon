<script setup lang="ts">
import { Loader2, Network, Pencil, Plus, Trash2 } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue';
import { toast } from 'vue-sonner';
import { useRouter } from 'vue-router';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import { useApiClientStore } from '@/stores/api-client';
import ProjectFormDialog from './components/ProjectFormDialog.vue';

const router = useRouter();
const store = useApiClientStore();

const createDialogOpen = ref(false);
const editDialogOpen = ref(false);
const editingProjectId = ref<string | null>(null);
const backendError = ref<string | null>(null);

interface PendingConfirm {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  destructive: boolean;
  resolve: ((value: boolean) => void) | null;
}

const pendingConfirm = reactive<PendingConfirm>({
  open: false,
  title: '',
  description: '',
  confirmLabel: '确认',
  cancelLabel: '取消',
  destructive: false,
  resolve: null,
});

function askConfirm(options: {
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}): Promise<boolean> {
  pendingConfirm.title = options.title;
  pendingConfirm.description = options.description;
  pendingConfirm.confirmLabel = options.confirmLabel ?? '确认';
  pendingConfirm.cancelLabel = options.cancelLabel ?? '取消';
  pendingConfirm.destructive = options.destructive ?? false;
  pendingConfirm.resolve = null;
  pendingConfirm.open = true;

  return new Promise<boolean>(resolve => {
    pendingConfirm.resolve = resolve;
  });
}

function onConfirmDialogUpdate(open: boolean): void {
  pendingConfirm.open = open;
}

function onConfirmDialogConfirm(): void {
  pendingConfirm.resolve?.(true);
  pendingConfirm.resolve = null;
}

function onConfirmDialogCancel(): void {
  pendingConfirm.resolve?.(false);
  pendingConfirm.resolve = null;
}

const sortedProjects = computed(() => [...store.projects].sort((a, b) => a.sort - b.sort));

async function bootstrap(): Promise<void> {
  backendError.value = null;
  try {
    await store.loadProjects();
  } catch (error) {
    console.error(error);
    backendError.value = error instanceof Error ? error.message : '加载项目失败';
  }
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
  const ok = await askConfirm({
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
          接口请求
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
      <AlertDescription>
        {{ backendError }}。请确认 Rust 端 API Client commands 已注册后重启应用。
      </AlertDescription>
    </Alert>

    <div v-if="store.isLoadingProjects" class="flex flex-1 items-center justify-center">
      <div class="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在加载项目…
      </div>
    </div>

    <div
      v-else-if="sortedProjects.length === 0"
      class="flex flex-1 items-center justify-center"
    >
      <Card class="w-full max-w-xl border-dashed bg-muted/10">
        <CardHeader class="items-center text-center">
          <div class="mx-auto flex size-14 items-center justify-center rounded-full border bg-background">
            <Network class="size-6 text-primary" />
          </div>
          <CardTitle>还没有 API 项目</CardTitle>
          <CardDescription>
            在上方输入项目名称并点击「新建项目」开始。
          </CardDescription>
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
          <button
            type="button"
            class="flex min-w-0 flex-1 items-start gap-3 text-left"
            @click="handleOpen(project.id)"
          >
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
            <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
              HTTP
            </Badge>
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
      :open="pendingConfirm.open"
      :title="pendingConfirm.title"
      :description="pendingConfirm.description"
      :confirm-label="pendingConfirm.confirmLabel"
      :cancel-label="pendingConfirm.cancelLabel"
      :destructive="pendingConfirm.destructive"
      @update:open="onConfirmDialogUpdate"
      @confirm="onConfirmDialogConfirm"
      @cancel="onConfirmDialogCancel"
    />

    <ProjectFormDialog
      mode="create"
      :open="createDialogOpen"
      @update:open="(value: boolean) => createDialogOpen = value"
      @submit="handleCreateProjectSubmit"
    />

    <ProjectFormDialog
      mode="edit"
      :open="editDialogOpen"
      :initial-name="editingProject?.name"
      :initial-description="editingProject?.description"
      @update:open="(value: boolean) => editDialogOpen = value"
      @submit="handleEditProjectSubmit"
    />
  </div>
</template>

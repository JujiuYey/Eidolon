<script setup lang="ts">
import { ChevronLeft, ChevronRight, Loader2, Network } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue';
import { toast } from 'vue-sonner';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import { useApiClientStore } from '@/stores/api-client';
import type { ApiClientHttpMethod, ApiClientKeyValueRow, ApiClientRequestBody } from '@/types/api-client';
import AiBodyGeneratorPanel from './components/AiBodyGeneratorPanel.vue';
import ApiClientTree from './components/ApiClientTree.vue';
import ApiRequestEditor from './components/ApiRequestEditor.vue';
import ApiResponsePanel from './components/ApiResponsePanel.vue';
import RequestHistoryPanel from './components/RequestHistoryPanel.vue';

const store = useApiClientStore();

const showAiPanel = ref(true);
const showHistoryPanel = ref(true);
const showResponsePanel = ref(true);
const includeCurrentBody = ref(false);

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

const newProjectName = ref('');
const newGroupName = ref('');
const newRequestName = ref('');
const newRequestMethod = ref<ApiClientHttpMethod>('GET');
const newRequestUrl = ref('');

const backendError = ref<string | null>(null);

const draft = computed(() => store.draft);
const execution = computed(() => store.execution);

const isSending = computed(() => execution.value?.isRunning ?? false);
const isResponseLoading = computed(() => execution.value?.responseView.kind === 'loading');

const responseView = computed(() => execution.value?.responseView ?? null);

const aiCandidate = computed(() => store.aiCandidate);
const aiModels = computed(() => store.aiAvailableModels);

function handleError(error: unknown, fallback: string): void {
  const message = error instanceof Error ? error.message : fallback;
  toast.error(message);
}

async function bootstrap(): Promise<void> {
  try {
    await store.loadProjects();
  } catch (error) {
    backendError.value = error instanceof Error ? error.message : '加载项目失败';
    return;
  }

  try {
    await store.loadAiModels();
  } catch {
    // ignore: model list is optional
  }
}

async function handleSelectProject(projectId: string): Promise<void> {
  if (store.isDirty) {
    const proceed = await promptUnsavedChange();
    if (!proceed) {
      return;
    }
  }

  try {
    await store.selectProject(projectId, { discardUnsaved: true });
  } catch (error) {
    handleError(error, '切换项目失败');
  }
}

async function handleSelectGroup(_groupId: string): Promise<void> {
  if (!draft.value) {
    return;
  }
  if (store.isDirty) {
    const proceed = await promptUnsavedChange();
    if (!proceed) {
      return;
    }
  }

  store.selectEnvironment(store.activeEnvironmentId);
}

async function handleSelectRequest(requestId: string): Promise<void> {
  if (store.isDirty) {
    const proceed = await promptUnsavedChange();
    if (!proceed) {
      return;
    }
  }

  try {
    await store.selectRequest(requestId, { discardUnsaved: true });
  } catch (error) {
    handleError(error, '切换请求失败');
  }
}

async function promptUnsavedChange(): Promise<boolean> {
  const proceed = await askConfirm({
    title: '当前请求存在未保存修改',
    description: '切换将放弃当前编辑。请先保存或继续编辑。',
    confirmLabel: '放弃修改',
    cancelLabel: '继续编辑',
    destructive: true,
  });
  if (!proceed) {
    return false;
  }
  store.discardDraftChanges();
  return true;
}

async function handleCreateProject(): Promise<void> {
  const name = newProjectName.value.trim();
  if (!name) {
    toast.error('请输入项目名称');
    return;
  }
  try {
    const project = await store.createProject(name, '');
    newProjectName.value = '';
    toast.success(`已创建项目 ${project.name}`);
    await handleSelectProject(project.id);
  } catch (error) {
    handleError(error, '创建项目失败');
  }
}

async function handleCreateGroup(_projectId: string): Promise<void> {
  const name = newGroupName.value.trim() || '默认分组';
  if (!store.activeProjectId) {
    return;
  }
  try {
    await store.createGroup(name);
    newGroupName.value = '';
    toast.success(`已创建分组 ${name}`);
  } catch (error) {
    handleError(error, '创建分组失败');
  }
}

async function handleCreateRequest(groupId: string): Promise<void> {
  const name = newRequestName.value.trim() || '未命名请求';
  try {
    const request = await store.createRequest({
      groupId,
      name,
      method: newRequestMethod.value,
      url: newRequestUrl.value,
    });
    newRequestName.value = '';
    newRequestUrl.value = '';
    newRequestMethod.value = 'GET';
    toast.success(`已创建请求 ${request.name}`);
    await handleSelectRequest(request.id);
  } catch (error) {
    handleError(error, '创建请求失败');
  }
}

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
    handleError(error, '删除项目失败');
  }
}

async function handleDeleteGroup(groupId: string): Promise<void> {
  const group = store.groups.find(item => item.id === groupId);
  const ok = await askConfirm({
    title: '删除分组',
    description: `确定删除分组「${group?.name ?? ''}」？该分组下的请求与历史也将一并删除。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  try {
    await store.deleteGroup(groupId);
    toast.success('分组已删除');
  } catch (error) {
    handleError(error, '删除分组失败');
  }
}

async function handleDeleteRequest(requestId: string): Promise<void> {
  const request = store.requests.find(item => item.id === requestId);
  const ok = await askConfirm({
    title: '删除请求',
    description: `确定删除请求「${request?.name ?? ''}」及其历史？`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  try {
    await store.deleteRequest(requestId);
    toast.success('请求已删除');
  } catch (error) {
    handleError(error, '删除请求失败');
  }
}

async function handleSaveDraft(): Promise<void> {
  try {
    await store.saveDraft();
    toast.success('请求已保存');
  } catch (error) {
    handleError(error, '保存失败');
  }
}

async function handleSend(): Promise<void> {
  try {
    await store.executeRequest();
  } catch (error) {
    handleError(error, '发送失败');
  }
}

async function handleCancelExecution(): Promise<void> {
  try {
    await store.cancelExecution();
    toast.success('已取消当前请求');
  } catch (error) {
    handleError(error, '取消失败');
  }
}

function handlePasteUrl(): void {
  if (!draft.value) {
    return;
  }
  store.pasteDraftUrl(draft.value.url);
  toast.success('已解析查询参数到 Params');
}

function handleSelectEnvironment(value: string): void {
  store.selectEnvironment(value || null);
}

function handleOpenAi(): void {
  showAiPanel.value = true;
}

async function handleGenerateAi(): Promise<void> {
  try {
    await store.generateAiBody({
      prompt: draft.value?.aiPrompt ?? '',
      reference: draft.value?.aiReference ?? '',
      includeCurrentBody: includeCurrentBody.value,
    });
  } catch (error) {
    handleError(error, '生成失败');
  }
}

async function handleCancelAi(): Promise<void> {
  try {
    await store.cancelAiGeneration();
  } catch (error) {
    handleError(error, '取消失败');
  }
}

function handleApplyAi(): void {
  const ok = store.applyAiCandidateToBody();
  if (!ok) {
    toast.error('候选内容未通过 JSON 校验');
  }
}

function handleRestoreHistory(historyId: string): void {
  store.applyHistoryToDraft(historyId);
}

async function handleClearHistory(): Promise<void> {
  const ok = await askConfirm({
    title: '清空历史',
    description: '确定清空当前请求的所有历史？',
    confirmLabel: '清空',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  try {
    await store.clearHistory();
    toast.success('历史已清空');
  } catch (error) {
    handleError(error, '清空历史失败');
  }
}

function updateDraftName(value: string): void {
  store.updateDraftName(value);
}

function updateDraftMethod(value: ApiClientHttpMethod): void {
  store.updateDraftMethod(value);
}

function updateDraftUrl(value: string): void {
  store.updateDraftUrl(value);
}

function updateDraftTimeoutMs(value: number): void {
  store.updateDraftTimeoutMs(value);
}

function updateDraftQuery(rows: ApiClientKeyValueRow[]): void {
  store.updateDraftQuery(rows);
}

function updateDraftHeaders(rows: ApiClientKeyValueRow[]): void {
  store.updateDraftHeaders(rows);
}

function updateDraftBody(body: ApiClientRequestBody): void {
  store.updateDraftBody(body);
}

function updateAiPrompt(value: string): void {
  if (!draft.value) {
    return;
  }
  store.updateDraftAiPrompt(value);
}

function updateAiReference(value: string): void {
  if (!draft.value) {
    return;
  }
  store.updateDraftAiReference(value);
}

function updateAiModel(key: string | null): void {
  store.selectAiModel(key);
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

const activeEnvironmentId = computed(() => store.activeEnvironmentId);
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex items-center justify-between border-b bg-card px-4 py-3">
      <div class="flex items-center gap-2">
        <Network class="size-5 text-primary" />
        <h1 class="text-base font-semibold">
          接口请求工具
        </h1>
        <span class="text-xs text-muted-foreground">项目、分组、请求与 AI 生成</span>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="ghost" size="sm" @click="showAiPanel = !showAiPanel">
          <component :is="showAiPanel ? ChevronRight : ChevronLeft" class="size-4" />
          AI 面板
        </Button>
        <Button variant="ghost" size="sm" @click="showHistoryPanel = !showHistoryPanel">
          <component :is="showHistoryPanel ? ChevronRight : ChevronLeft" class="size-4" />
          历史
        </Button>
      </div>
    </div>

    <Alert v-if="backendError" variant="destructive" class="m-4">
      <AlertTitle>无法连接到接口请求后端</AlertTitle>
      <AlertDescription>
        {{ backendError }}。请确认 Rust 端 API Client commands 已注册后重启应用。
      </AlertDescription>
    </Alert>

    <div class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_minmax(0,360px)]">
      <ApiClientTree
        :projects="store.projects"
        :groups="store.groups"
        :requests="store.requestSearchResults"
        :search-keyword="store.requestListFilter"
        :active-project-id="store.activeProjectId"
        :active-group-id="store.activeGroupId"
        :active-request-id="store.activeRequestId"
        :is-loading-projects="store.isLoadingProjects"
        :is-loading-groups="store.isLoadingGroups"
        :is-loading-requests="store.isLoadingRequests"
        :deleting-project-id="store.isDeletingProject"
        :deleting-group-id="store.isDeletingGroup"
        :deleting-request-id="store.isDeletingRequest"
        @update:search-keyword="store.setSearchKeyword"
        @select-project="handleSelectProject"
        @select-group="handleSelectGroup"
        @select-request="handleSelectRequest"
        @create-project="handleCreateProject"
        @create-group="handleCreateGroup"
        @create-request="handleCreateRequest"
        @delete-project="handleDeleteProject"
        @delete-group="handleDeleteGroup"
        @delete-request="handleDeleteRequest"
      />

      <ScrollArea class="h-full">
        <div class="flex flex-col gap-4 p-4">
          <Card>
            <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
              <div>
                <CardTitle>新建内容</CardTitle>
                <CardDescription>从当前选中的项目或分组快速创建</CardDescription>
              </div>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="grid grid-cols-1 gap-2 md:grid-cols-[1fr_auto]">
                <div class="flex flex-col gap-1">
                  <Label>新建项目</Label>
                  <Input v-model="newProjectName" placeholder="项目名称" />
                </div>
                <Button class="md:self-end" @click="handleCreateProject">
                  新建项目
                </Button>
              </div>
              <div class="grid grid-cols-1 gap-2 md:grid-cols-[1fr_auto]">
                <div class="flex flex-col gap-1">
                  <Label>新建分组（当前项目）</Label>
                  <Input v-model="newGroupName" placeholder="分组名称" />
                </div>
                <Button class="md:self-end" :disabled="!store.activeProjectId" @click="store.activeProjectId && handleCreateGroup(store.activeProjectId)">
                  新建分组
                </Button>
              </div>
              <div class="grid grid-cols-1 gap-2 md:grid-cols-[1fr_auto]">
                <div class="flex flex-col gap-1">
                  <Label>新建请求</Label>
                  <Input v-model="newRequestName" placeholder="请求名称" />
                </div>
                <div class="flex flex-col gap-1">
                  <Label>方法</Label>
                  <select v-model="newRequestMethod" class="rounded-md border bg-background px-2 py-1 text-sm">
                    <option v-for="method of (['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as ApiClientHttpMethod[])" :key="method" :value="method">
                      {{ method }}
                    </option>
                  </select>
                </div>
                <div class="flex flex-col gap-1">
                  <Label>URL</Label>
                  <Input v-model="newRequestUrl" placeholder="https://example.com/path" />
                </div>
                <Button class="md:self-end" :disabled="!store.activeGroupId" @click="store.activeGroupId && handleCreateRequest(store.activeGroupId)">
                  新建请求
                </Button>
              </div>
            </CardContent>
          </Card>

          <ApiRequestEditor
            v-if="draft"
            :is-sending="isSending"
            :is-saving="store.isSavingRequest"
            :is-dirty="store.isDirty"
            :draft-name="draft.name"
            :draft-method="draft.method"
            :draft-url="draft.url"
            :draft-timeout-ms="draft.timeoutMs"
            :draft-query="draft.query"
            :draft-headers="draft.headers"
            :draft-body="draft.body"
            :environments="store.environments"
            :active-environment-id="activeEnvironmentId"
            @update:draft-name="updateDraftName"
            @update:draft-method="updateDraftMethod"
            @update:draft-url="updateDraftUrl"
            @update:draft-timeout-ms="updateDraftTimeoutMs"
            @update:draft-query="updateDraftQuery"
            @update:draft-headers="updateDraftHeaders"
            @update:draft-body="updateDraftBody"
            @save="handleSaveDraft"
            @send="handleSend"
            @cancel="handleCancelExecution"
            @paste-url="handlePasteUrl"
            @open-ai="handleOpenAi"
            @select-environment="handleSelectEnvironment"
          />

          <div v-else-if="store.activeProjectId" class="flex flex-col items-center justify-center rounded-md border border-dashed bg-muted/10 px-6 py-12 text-center text-sm text-muted-foreground">
            <Network class="mb-2 size-6 text-muted-foreground" />
            请在左侧选择或新建一个请求
          </div>

          <div v-else-if="store.isLoadingProjects" class="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 class="size-4 animate-spin" /> 加载项目…
          </div>

          <div v-else class="flex flex-col items-center justify-center rounded-md border border-dashed bg-muted/10 px-6 py-12 text-center text-sm text-muted-foreground">
            <Network class="mb-2 size-6 text-muted-foreground" />
            请先创建一个项目以开始
          </div>

          <ApiResponsePanel
            v-if="showResponsePanel && draft"
            :response="responseView"
            :is-loading="isResponseLoading"
          />

          <RequestHistoryPanel
            v-if="showHistoryPanel && draft"
            :histories="store.history"
            :is-loading="store.isLoadingHistory"
            :is-clearing="store.isClearingHistory"
            @restore="handleRestoreHistory"
            @clear="handleClearHistory"
          />
        </div>
      </ScrollArea>

      <ScrollArea v-if="showAiPanel" class="h-full border-l bg-card">
        <div class="flex flex-col gap-4 p-4">
          <AiBodyGeneratorPanel
            v-if="draft"
            :candidate="aiCandidate"
            :is-generating="store.isGeneratingAi"
            :current-body="draft.body"
            :prompt="draft.aiPrompt"
            :reference="draft.aiReference"
            :include-current-body="includeCurrentBody"
            :models="aiModels"
            :selected-model-key="store.aiSelectedModelKey"
            @update:prompt="updateAiPrompt"
            @update:reference="updateAiReference"
            @update:include-current-body="includeCurrentBody = $event"
            @update:selected-model-key="updateAiModel"
            @generate="handleGenerateAi"
            @cancel="handleCancelAi"
            @apply="handleApplyAi"
            @edit="store.editAiCandidateContent($event)"
          />

          <Card v-else>
            <CardHeader>
              <CardTitle>AI 生成</CardTitle>
              <CardDescription>选择请求后启用 AI 请求体生成。</CardDescription>
            </CardHeader>
          </Card>
        </div>
      </ScrollArea>
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
  </div>
</template>

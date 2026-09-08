<script setup lang="ts">
import { ChevronLeft, ChevronRight, FolderPlus, Network, Plus } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch as vueWatch } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import { useApiClientStore } from '@/stores/api-client';
import type { ApiClientHttpMethod, ApiClientKeyValueRow, ApiClientRequestBody } from '@/types/api-client';
import AiBodyGeneratorPanel from './components/AiBodyGeneratorPanel.vue';
import ApiClientTree from './components/ApiClientTree.vue';
import ApiRequestEditor from './components/ApiRequestEditor.vue';
import ApiResponsePanel from './components/ApiResponsePanel.vue';
import CreateGroupDialog from './components/CreateGroupDialog.vue';
import CreateRequestDialog from './components/CreateRequestDialog.vue';
import RequestHistoryPanel from './components/RequestHistoryPanel.vue';

const props = defineProps<{
  projectId: string;
}>();

const router = useRouter();
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

const createGroupDialogOpen = ref(false);
const createRequestDialogOpen = ref(false);
const createRequestGroupName = ref('');

const draft = computed(() => store.draft);
const execution = computed(() => store.execution);

const isSending = computed(() => execution.value?.isRunning ?? false);
const isResponseLoading = computed(() => execution.value?.responseView.kind === 'loading');

const responseView = computed(() => execution.value?.responseView ?? null);

const aiCandidate = computed(() => store.aiCandidate);
const aiCurrentModelLabel = computed(() => store.aiCurrentModelLabel);

function handleError(error: unknown, fallback: string): void {
  const message = error instanceof Error ? error.message : fallback;
  toast.error(message);
}

async function bootstrapForProject(): Promise<void> {
  if (store.activeProjectId !== props.projectId) {
    try {
      await store.selectProject(props.projectId, { discardUnsaved: true });
    } catch (error) {
      handleError(error, '切换项目失败');
      return;
    }
  }
  try {
    await store.loadAiModels();
  } catch {
    // ignore: no backend list command; current model is reported per generation
  }
}

async function handleSelectProject(projectId: string): Promise<void> {
  if (store.isDirty) {
    const proceed = await promptUnsavedChange();
    if (!proceed) {
      return;
    }
  }
  if (projectId !== props.projectId) {
    void router.push('/api-client');
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

function openCreateGroupDialog(): void {
  createGroupDialogOpen.value = true;
}

async function submitCreateGroup(payload: { name: string }): Promise<void> {
  if (!store.activeProjectId) {
    return;
  }
  try {
    await store.createGroup(payload.name);
    createGroupDialogOpen.value = false;
    toast.success(`已创建分组 ${payload.name}`);
  } catch (error) {
    handleError(error, '创建分组失败');
  }
}

function openCreateRequestDialog(groupId: string, groupName: string): void {
  pendingCreateRequestGroupId.value = groupId;
  createRequestGroupName.value = groupName;
  createRequestDialogOpen.value = true;
}

function handleTreeCreateRequest(payload: { groupId: string; groupName: string }): void {
  openCreateRequestDialog(payload.groupId, payload.groupName);
}

const pendingCreateRequestGroupId = ref<string | null>(null);

async function submitCreateRequest(payload: { name: string; method: ApiClientHttpMethod; url: string }): Promise<void> {
  const groupId = pendingCreateRequestGroupId.value;
  if (!groupId) {
    return;
  }
  try {
    const request = await store.createRequest({
      groupId,
      name: payload.name,
      method: payload.method,
      url: payload.url,
    });
    createRequestDialogOpen.value = false;
    pendingCreateRequestGroupId.value = null;
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
    void router.push('/api-client');
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

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (store.isDirty) {
    event.preventDefault();
    event.returnValue = '';
  }
}

function handleBack(): void {
  void router.push('/api-client');
}

vueWatch(() => props.projectId, async next => {
  if (next) {
    await bootstrapForProject();
  }
}, { immediate: true });

onMounted(() => {
  window.addEventListener('beforeunload', handleBeforeUnload);
});

onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', handleBeforeUnload);
});

const activeEnvironmentId = computed(() => store.activeEnvironmentId);
const activeProject = computed(() => store.activeProject);
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex items-center justify-between border-b bg-card px-4 py-3">
      <div class="flex items-center gap-2">
        <Button variant="ghost" size="sm" @click="handleBack">
          <ChevronLeft class="size-4" />
          返回项目
        </Button>
        <Network class="size-5 text-primary" />
        <h1 class="text-base font-semibold">
          {{ activeProject?.name ?? '接口请求工具' }}
        </h1>
        <span class="text-xs text-muted-foreground">分组、请求、响应与 AI 生成</span>
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

    <Alert v-if="!store.activeProjectId" variant="destructive" class="m-4">
      <AlertTitle>项目不可用</AlertTitle>
      <AlertDescription>
        请返回项目列表后重新进入。
      </AlertDescription>
    </Alert>

    <div v-else class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_minmax(0,360px)]">
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
        @create-group="openCreateGroupDialog"
        @create-request="handleTreeCreateRequest"
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
                <CardDescription>使用弹窗在当前项目下创建分组或请求。</CardDescription>
              </div>
              <div class="flex gap-2">
                <Button variant="outline" size="sm" @click="openCreateGroupDialog">
                  <FolderPlus class="size-4" />
                  新建分组
                </Button>
                <Button size="sm" :disabled="!store.activeGroupId" @click="store.activeGroupId && openCreateRequestDialog(store.activeGroupId, store.groups.find(item => item.id === store.activeGroupId)?.name ?? '')">
                  <Plus class="size-4" />
                  新建请求
                </Button>
              </div>
            </CardHeader>
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

          <div v-else class="flex flex-col items-center justify-center rounded-md border border-dashed bg-muted/10 px-6 py-12 text-center text-sm text-muted-foreground">
            <Network class="mb-2 size-6 text-muted-foreground" />
            请在左侧选择或新建一个请求
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
            :current-model-label="aiCurrentModelLabel"
            @update:prompt="updateAiPrompt"
            @update:reference="updateAiReference"
            @update:include-current-body="includeCurrentBody = $event"
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

    <Spinner v-if="store.isLoadingGroups || store.isLoadingRequests" class="pointer-events-none fixed right-6 top-6" />

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

    <CreateGroupDialog
      :open="createGroupDialogOpen"
      :project-name="store.activeProject?.name"
      @update:open="(value: boolean) => createGroupDialogOpen = value"
      @submit="submitCreateGroup"
    />

    <CreateRequestDialog
      :open="createRequestDialogOpen"
      :group-name="createRequestGroupName"
      @update:open="(value: boolean) => createRequestDialogOpen = value"
      @submit="submitCreateRequest"
    />
  </div>
</template>

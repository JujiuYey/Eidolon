<script setup lang="ts">
import { ChevronLeft, ChevronRight, FolderPlus, Network, Plus } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref, watch as vueWatch } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import { useConfirm } from '@/composables/use-confirm';
import { useApiClientStore } from '@/stores/api-client';
import type { ApiClientHttpMethod } from '@/types/api-client';
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
const includeCurrentBody = ref(false);

const createGroupDialogOpen = ref(false);
const createRequestDialogOpen = ref(false);
const createRequestGroupName = ref('');

const confirm = useConfirm();

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

async function runAsync(task: () => Promise<void>, fallbackMessage: string): Promise<void> {
  try {
    await task();
  } catch (error) {
    handleError(error, fallbackMessage);
  }
}

async function bootstrapForProject(): Promise<void> {
  if (store.activeProjectId !== props.projectId) {
    try {
      await store.selectProject(props.projectId, { discardUnsaved: true });
    } catch (error) {
      handleError(error, '切换项目失败');
    }
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
  const proceed = await confirm.ask({
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

async function confirmAndDelete(options: {
  title: string;
  description: string;
  run: () => Promise<void>;
  success: string;
  errorMessage: string;
  after?: () => void;
}): Promise<void> {
  const ok = await confirm.ask({
    title: options.title,
    description: options.description,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  await runAsync(async () => {
    await options.run();
    toast.success(options.success);
    options.after?.();
  }, options.errorMessage);
}

async function handleDeleteProject(projectId: string): Promise<void> {
  const project = store.projects.find(item => item.id === projectId);
  await confirmAndDelete({
    title: '删除项目',
    description: `确定删除项目「${project?.name ?? ''}」？此操作将一并删除该项目下的所有请求、分组和环境。`,
    run: () => store.deleteProject(projectId),
    success: '项目已删除',
    errorMessage: '删除项目失败',
    after: () => {
      void router.push('/api-client');
    },
  });
}

async function handleDeleteGroup(groupId: string): Promise<void> {
  const group = store.groups.find(item => item.id === groupId);
  await confirmAndDelete({
    title: '删除分组',
    description: `确定删除分组「${group?.name ?? ''}」？该分组下的请求与历史也将一并删除。`,
    run: () => store.deleteGroup(groupId),
    success: '分组已删除',
    errorMessage: '删除分组失败',
  });
}

async function handleDeleteRequest(requestId: string): Promise<void> {
  const request = store.requests.find(item => item.id === requestId);
  await confirmAndDelete({
    title: '删除请求',
    description: `确定删除请求「${request?.name ?? ''}」及其历史？`,
    run: () => store.deleteRequest(requestId),
    success: '请求已删除',
    errorMessage: '删除请求失败',
  });
}

async function handleSaveDraft(): Promise<void> {
  await runAsync(async () => {
    await store.saveDraft();
    toast.success('请求已保存');
  }, '保存失败');
}

async function handleSend(): Promise<void> {
  await runAsync(() => store.executeRequest(), '发送失败');
}

async function handleCancelExecution(): Promise<void> {
  await runAsync(async () => {
    await store.cancelExecution();
    toast.success('已取消当前请求');
  }, '取消失败');
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
  await runAsync(() => store.generateAiBody({
    prompt: draft.value?.aiPrompt ?? '',
    reference: draft.value?.aiReference ?? '',
    includeCurrentBody: includeCurrentBody.value,
  }), '生成失败');
}

async function handleCancelAi(): Promise<void> {
  await runAsync(() => store.cancelAiGeneration(), '取消失败');
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
  const ok = await confirm.ask({
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
  <div class="flex h-full min-h-0 flex-col overflow-hidden">
    <div class="flex shrink-0 items-center justify-between border-b bg-card px-4 py-3">
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

    <Alert v-if="!store.activeProjectId" variant="destructive" class="m-4 shrink-0">
      <AlertTitle>项目不可用</AlertTitle>
      <AlertDescription>
        请返回项目列表后重新进入。
      </AlertDescription>
    </Alert>

    <div v-else class="grid min-h-0 flex-1 grid-cols-[280px_minmax(0,1fr)_360px] grid-rows-[minmax(0,1fr)] overflow-hidden">
      <div class="min-h-0 overflow-hidden border-r bg-card">
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
      </div>

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
            @update:draft-name="store.updateDraftName"
            @update:draft-method="store.updateDraftMethod"
            @update:draft-url="store.updateDraftUrl"
            @update:draft-timeout-ms="store.updateDraftTimeoutMs"
            @update:draft-query="store.updateDraftQuery"
            @update:draft-headers="store.updateDraftHeaders"
            @update:draft-body="store.updateDraftBody"
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
            v-if="draft"
            :response="responseView"
            :is-loading="isResponseLoading"
          />
        </div>
      </ScrollArea>

      <div class="flex min-h-0 flex-col overflow-hidden border-l bg-card">
        <ScrollArea v-if="showHistoryPanel" class="min-h-0 shrink basis-1/2 border-b">
          <div class="flex flex-col gap-4 p-4">
            <RequestHistoryPanel
              v-if="draft"
              :histories="store.history"
              :is-loading="store.isLoadingHistory"
              :is-clearing="store.isClearingHistory"
              @restore="handleRestoreHistory"
              @clear="handleClearHistory"
            />
            <Card v-else>
              <CardHeader>
                <CardTitle>历史</CardTitle>
                <CardDescription>选择请求后查看最近 100 次执行快照。</CardDescription>
              </CardHeader>
            </Card>
          </div>
        </ScrollArea>

        <ScrollArea v-if="showAiPanel" class="min-h-0 shrink basis-1/2 grow">
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
    </div>

    <Spinner v-if="store.isLoadingGroups || store.isLoadingRequests" class="pointer-events-none fixed right-6 top-6" />

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

<script setup lang="ts">
import { Network } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref, watch as vueWatch } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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
import EnvironmentManagerDialog from './components/EnvironmentManagerDialog.vue';
import WorkspaceToolbar from './components/WorkspaceToolbar.vue';

const props = defineProps<{
  projectId: string;
}>();

const router = useRouter();
const store = useApiClientStore();

const showAiPanel = ref(false);
const showHistoryPanel = ref(false);
const includeCurrentBody = ref(false);

const createGroupDialogOpen = ref(false);
const createRequestDialogOpen = ref(false);
const environmentDialogOpen = ref(false);
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

function handleSelectGroup(groupId: string): void {
  store.selectGroup(groupId);
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
const pendingCreateGroupParent = ref<{ id: string; name: string } | null>(null);
const renamingGroupId = ref<string | null>(null);

function openCreateGroupDialog(parentContext?: { id: string; name: string }): void {
  pendingCreateGroupParent.value = parentContext ?? null;
  createGroupDialogOpen.value = true;
}

function handleTreeCreateGroup(payload: { parentGroupId: string | null; parentGroupName: string | null }): void {
  if (payload.parentGroupId && payload.parentGroupName) {
    openCreateGroupDialog({ id: payload.parentGroupId, name: payload.parentGroupName });
  } else {
    openCreateGroupDialog();
  }
}

async function submitCreateGroup(payload: { name: string; parentGroupId: string | null }): Promise<void> {
  if (!store.activeProjectId) {
    return;
  }
  try {
    await store.createGroup({
      projectId: store.activeProjectId,
      name: payload.name,
      parentGroupId: payload.parentGroupId,
    });
    createGroupDialogOpen.value = false;
    pendingCreateGroupParent.value = null;
    toast.success(`已创建分组 ${payload.name}`);
  } catch (error) {
    handleError(error, '创建分组失败');
  }
}

async function handleRenameGroup(payload: { groupId: string; name: string }): Promise<void> {
  renamingGroupId.value = payload.groupId;
  try {
    await store.renameGroup(payload.groupId, payload.name);
    toast.success('分组已重命名');
  } catch (error) {
    handleError(error, '重命名分组失败');
  } finally {
    renamingGroupId.value = null;
  }
}

function onCreateGroupDialogOpenChange(open: boolean): void {
  createGroupDialogOpen.value = open;
  if (!open) {
    pendingCreateGroupParent.value = null;
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
  showHistoryPanel.value = false;
}

function handleToggleAiPanel(): void {
  showAiPanel.value = !showAiPanel.value;
  if (showAiPanel.value) {
    showHistoryPanel.value = false;
  }
}

function handleToggleHistoryPanel(): void {
  showHistoryPanel.value = !showHistoryPanel.value;
  if (showHistoryPanel.value) {
    showAiPanel.value = false;
  }
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
    <WorkspaceToolbar
      :project-name="activeProject?.name ?? ''"
      :show-ai-panel="showAiPanel"
      :show-history-panel="showHistoryPanel"
      @back="handleBack"
      @toggle-ai="handleToggleAiPanel"
      @toggle-history="handleToggleHistoryPanel"
      @open-environments="environmentDialogOpen = true"
    />

    <Alert v-if="!store.activeProjectId" variant="destructive" class="m-4 shrink-0">
      <AlertTitle>项目不可用</AlertTitle>
      <AlertDescription>
        请返回项目列表后重新进入。
      </AlertDescription>
    </Alert>

    <div
      v-else
      class="grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)] overflow-hidden"
      :class="showHistoryPanel || showAiPanel ? 'grid-cols-[280px_minmax(0,1fr)_360px]' : 'grid-cols-[280px_minmax(0,1fr)]'"
    >
      <div class="min-h-0 overflow-hidden border-r bg-card">
        <ApiClientTree
          :project-name="store.activeProject?.name ?? ''"
          :groups="store.groups"
          :requests="store.requestSearchResults"
          :search-keyword="store.requestListFilter"
          :active-group-id="store.activeGroupId"
          :active-request-id="store.activeRequestId"
          :is-loading-groups="store.isLoadingGroups"
          :is-loading-requests="store.isLoadingRequests"
          :deleting-group-id="store.isDeletingGroup"
          :deleting-request-id="store.isDeletingRequest"
          :renaming-group-id="renamingGroupId"
          @select-group="handleSelectGroup"
          @select-request="handleSelectRequest"
          @create-group="handleTreeCreateGroup"
          @create-request="handleTreeCreateRequest"
          @rename-group="handleRenameGroup"
          @delete-group="handleDeleteGroup"
          @delete-request="handleDeleteRequest"
        />
      </div>

      <ScrollArea class="h-full">
        <div class="flex flex-col gap-4 p-4">
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

      <div v-if="showHistoryPanel || showAiPanel" class="flex min-h-0 flex-1 flex-col overflow-hidden bg-card">
        <div v-if="showHistoryPanel && !showAiPanel" class="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden p-4">
          <RequestHistoryPanel
            v-if="draft"
            class="min-h-0 flex-1"
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

        <div v-else-if="showAiPanel" class="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden p-4">
          <AiBodyGeneratorPanel
            v-if="draft"
            class="min-h-0 flex-1"
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
      :parent-group-id="pendingCreateGroupParent?.id ?? null"
      :parent-group-name="pendingCreateGroupParent?.name ?? null"
      @update:open="onCreateGroupDialogOpenChange"
      @submit="submitCreateGroup"
    />
    <CreateRequestDialog
      :open="createRequestDialogOpen"
      :group-name="createRequestGroupName"
      @update:open="(value: boolean) => createRequestDialogOpen = value"
      @submit="submitCreateRequest"
    />

    <EnvironmentManagerDialog
      :open="environmentDialogOpen"
      @update:open="(value: boolean) => environmentDialogOpen = value"
    />
  </div>
</template>

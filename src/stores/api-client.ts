import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as apiClientService from '@/services/api-client';
import type {
  ApiAiModelOption,
  ApiExecuteResult,
} from '@/services/api-client';
import type {
  ApiClientAiCandidate,
  ApiClientEnvironment,
  ApiClientGroup,
  ApiClientHttpMethod,
  ApiClientKeyValueRow,
  ApiClientProject,
  ApiClientRequest,
  ApiClientRequestBody,
  ApiClientRequestHistory,
  ApiClientRequestSnapshot,
  ApiClientResponseView,
} from '@/types/api-client';
import {
  applyCandidateToBody,
  buildAiCandidate,
  buildAiCandidateFailed,
  buildAiCandidateGenerating,
  buildAiContextPayload,
} from '@/views/api-client/utils/ai-helpers';
import {
  buildEnvironmentLookup,
  buildRequestSnapshot,
  cloneBody,
  cloneRows,
  collectMissingVariablesInBody,
  collectMissingVariablesInRows,
  createEmptyRow,
  createId,
  diffDraftAgainstSaved,
  maskHeadersForHistory,
  resolveExecutionUrl,
  splitUrlIntoBaseAndParams,
  tryFormatJson,
} from '@/views/api-client/utils/request-helpers';
import { buildResponseView, classifyResponseKind } from '@/views/api-client/utils/response-helpers';

interface RequestDraft {
  id: string;
  name: string;
  method: ApiClientHttpMethod;
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  timeoutMs: number;
  aiPrompt: string;
  aiReference: string;
  savedName: string;
}

interface RequestExecution {
  executionId: string;
  requestId: string;
  isRunning: boolean;
  result: ApiExecuteResult | null;
  responseView: ApiClientResponseView;
}

export type ApiClientService = typeof apiClientService;

export interface CreateApiClientStoreOptions {
  service?: ApiClientService;
}

export function createApiClientStore(options: CreateApiClientStoreOptions = {}) {
  const service = options.service ?? apiClientService;

  const projects = ref<ApiClientProject[]>([]);
  const groups = ref<ApiClientGroup[]>([]);
  const requests = ref<ApiClientRequest[]>([]);
  const environments = ref<ApiClientEnvironment[]>([]);

  const activeProjectId = ref<string | null>(null);
  const activeGroupId = ref<string | null>(null);
  const activeRequestId = ref<string | null>(null);
  const activeEnvironmentId = ref<string | null>(null);

  const draft = ref<RequestDraft | null>(null);
  const lastSavedRequestSnapshot = ref<ApiClientRequestSnapshot | null>(null);

  const isLoadingProjects = ref(false);
  const isLoadingGroups = ref(false);
  const isLoadingRequests = ref(false);
  const isLoadingEnvironments = ref(false);
  const isSavingRequest = ref(false);
  const isDeletingProject = ref<string | null>(null);
  const isDeletingGroup = ref<string | null>(null);
  const isDeletingRequest = ref<string | null>(null);

  const execution = ref<RequestExecution | null>(null);
  const executionAbortController = ref<AbortController | null>(null);

  const aiCandidate = ref<ApiClientAiCandidate | null>(null);
  const aiAbortController = ref<AbortController | null>(null);
  const aiAvailableModels = ref<ApiAiModelOption[]>([]);
  const aiSelectedModelKey = ref<string | null>(null);
  const isGeneratingAi = ref(false);

  const history = ref<ApiClientRequestHistory[]>([]);
  const isLoadingHistory = ref(false);
  const isClearingHistory = ref(false);

  const requestListFilter = ref('');

  const searchKeyword = computed(() => requestListFilter.value.trim().toLowerCase());

  const activeProject = computed(() => projects.value.find(project => project.id === activeProjectId.value) ?? null);
  const activeGroup = computed(() => groups.value.find(group => group.id === activeGroupId.value) ?? null);
  const activeEnvironment = computed(() => environments.value.find(env => env.id === activeEnvironmentId.value) ?? null);
  const activeRequest = computed(() => requests.value.find(request => request.id === activeRequestId.value) ?? null);

  const requestSearchResults = computed(() => {
    const keyword = searchKeyword.value;
    if (!keyword) {
      return requests.value;
    }
    return requests.value.filter(request => {
      return request.name.toLowerCase().includes(keyword);
    });
  });

  const isDirty = computed(() => {
    if (!draft.value) {
      return false;
    }
    if (draft.value.name.trim() === '') {
      return true;
    }
    return diffDraftAgainstSaved({
      saved: lastSavedRequestSnapshot.value,
      savedName: draft.value.savedName,
      draft: {
        name: draft.value.name,
        method: draft.value.method,
        url: draft.value.url,
        query: draft.value.query,
        headers: draft.value.headers,
        body: draft.value.body,
        timeoutMs: draft.value.timeoutMs,
      },
    }).isDirty;
  });

  async function loadProjects(): Promise<void> {
    isLoadingProjects.value = true;
    try {
      projects.value = await service.listApiProjects();
    } finally {
      isLoadingProjects.value = false;
    }
  }

  async function loadGroups(projectId: string): Promise<void> {
    isLoadingGroups.value = true;
    try {
      groups.value = await service.listApiGroups(projectId);
    } finally {
      isLoadingGroups.value = false;
    }
  }

  async function loadRequests(projectId: string): Promise<void> {
    isLoadingRequests.value = true;
    try {
      requests.value = await service.listAllApiRequests(projectId);
    } finally {
      isLoadingRequests.value = false;
    }
  }

  async function loadEnvironments(projectId: string): Promise<void> {
    isLoadingEnvironments.value = true;
    try {
      environments.value = await service.listApiEnvironments(projectId);
    } finally {
      isLoadingEnvironments.value = false;
    }
  }

  async function selectProject(projectId: string, options: { discardUnsaved?: boolean } = {}): Promise<void> {
    if (activeProjectId.value === projectId) {
      return;
    }
    if (!options.discardUnsaved && draft.value && isDirty.value) {
      throw new Error('当前请求存在未保存修改');
    }

    activeProjectId.value = projectId;
    activeGroupId.value = null;
    activeRequestId.value = null;
    draft.value = null;
    lastSavedRequestSnapshot.value = null;
    clearExecutionState();
    clearAiState();

    await Promise.all([
      loadGroups(projectId),
      loadRequests(projectId),
      loadEnvironments(projectId),
    ]);

    activeEnvironmentId.value = environments.value[0]?.id ?? null;
  }

  async function createProject(name: string, description: string): Promise<ApiClientProject> {
    const project = await service.createApiProject(name, description);
    projects.value = [...projects.value, project];
    return project;
  }

  async function renameProject(projectId: string, name: string): Promise<ApiClientProject> {
    const project = await service.renameApiProject(projectId, name);
    projects.value = projects.value.map(existing => (existing.id === projectId ? project : existing));
    return project;
  }

  async function updateProject(projectId: string, name: string, description: string): Promise<ApiClientProject> {
    const project = await service.updateApiProject(projectId, name, description);
    projects.value = projects.value.map(existing => (existing.id === projectId ? project : existing));
    return project;
  }

  async function deleteProject(projectId: string): Promise<void> {
    isDeletingProject.value = projectId;
    try {
      await service.deleteApiProject(projectId);
      projects.value = projects.value.filter(existing => existing.id !== projectId);
      if (activeProjectId.value === projectId) {
        activeProjectId.value = null;
        activeGroupId.value = null;
        activeRequestId.value = null;
        draft.value = null;
        lastSavedRequestSnapshot.value = null;
        groups.value = [];
        requests.value = [];
        environments.value = [];
        clearExecutionState();
        clearAiState();
      }
    } finally {
      isDeletingProject.value = null;
    }
  }

  async function createGroup(name: string): Promise<ApiClientGroup> {
    if (!activeProjectId.value) {
      throw new Error('请先选择项目');
    }
    const group = await service.createApiGroup(activeProjectId.value, name);
    groups.value = [...groups.value, group];
    return group;
  }

  async function renameGroup(groupId: string, name: string): Promise<ApiClientGroup> {
    const group = await service.renameApiGroup(groupId, name);
    groups.value = groups.value.map(existing => (existing.id === groupId ? group : existing));
    return group;
  }

  async function deleteGroup(groupId: string): Promise<void> {
    isDeletingGroup.value = groupId;
    try {
      await service.deleteApiGroup(groupId);
      groups.value = groups.value.filter(existing => existing.id !== groupId);
      requests.value = requests.value.filter(request => request.groupId !== groupId);
      if (activeGroupId.value === groupId) {
        activeGroupId.value = null;
        if (activeRequestId.value) {
          activeRequestId.value = null;
          draft.value = null;
          lastSavedRequestSnapshot.value = null;
          clearExecutionState();
          clearAiState();
        }
      }
    } finally {
      isDeletingGroup.value = null;
    }
  }

  async function selectRequest(requestId: string, options: { discardUnsaved?: boolean } = {}): Promise<void> {
    if (activeRequestId.value === requestId) {
      return;
    }
    if (!options.discardUnsaved && draft.value && isDirty.value) {
      throw new Error('当前请求存在未保存修改');
    }

    const request = requests.value.find(item => item.id === requestId);
    if (!request) {
      return;
    }

    activeRequestId.value = requestId;
    activeGroupId.value = request.groupId;
    draft.value = buildDraftFromRequest(request);
    lastSavedRequestSnapshot.value = buildRequestSnapshot({
      method: request.method,
      url: request.url,
      query: request.query,
      headers: request.headers,
      body: request.body,
      timeoutMs: request.timeoutMs,
      environmentName: activeEnvironment.value?.name ?? null,
    });
    clearExecutionState();
    clearAiState();
    await loadHistory(requestId);
  }

  async function createRequest(input: {
    groupId: string;
    name: string;
    method: ApiClientHttpMethod;
    url: string;
  }): Promise<ApiClientRequest> {
    const request = await service.createApiRequest(input);
    requests.value = [...requests.value, request];
    return request;
  }

  async function saveDraft(): Promise<ApiClientRequest | null> {
    if (!draft.value || !activeRequestId.value) {
      return null;
    }
    const existing = requests.value.find(request => request.id === activeRequestId.value);
    if (!existing) {
      return null;
    }

    isSavingRequest.value = true;
    try {
      const updated: ApiClientRequest = {
        ...existing,
        name: draft.value.name,
        method: draft.value.method,
        url: draft.value.url,
        query: cloneRows(draft.value.query),
        headers: cloneRows(draft.value.headers),
        body: cloneBody(draft.value.body),
        timeoutMs: draft.value.timeoutMs,
        aiPrompt: draft.value.aiPrompt,
        aiReference: draft.value.aiReference,
      };
      const persisted = await service.saveApiRequest(updated);
      requests.value = requests.value.map(item => (item.id === persisted.id ? persisted : item));

      draft.value = buildDraftFromRequest(persisted);
      lastSavedRequestSnapshot.value = buildRequestSnapshot({
        method: persisted.method,
        url: persisted.url,
        query: persisted.query,
        headers: persisted.headers,
        body: persisted.body,
        timeoutMs: persisted.timeoutMs,
        environmentName: activeEnvironment.value?.name ?? null,
      });
      return persisted;
    } finally {
      isSavingRequest.value = false;
    }
  }

  async function duplicateRequest(requestId: string): Promise<ApiClientRequest> {
    const request = await service.duplicateApiRequest(requestId);
    requests.value = [...requests.value, request];
    return request;
  }

  async function deleteRequest(requestId: string): Promise<void> {
    isDeletingRequest.value = requestId;
    try {
      await service.deleteApiRequest(requestId);
      requests.value = requests.value.filter(existing => existing.id !== requestId);
      if (activeRequestId.value === requestId) {
        activeRequestId.value = null;
        draft.value = null;
        lastSavedRequestSnapshot.value = null;
        clearExecutionState();
        clearAiState();
        history.value = [];
      }
    } finally {
      isDeletingRequest.value = null;
    }
  }

  async function moveRequest(requestId: string, groupId: string, sort: number): Promise<ApiClientRequest> {
    const request = await service.moveApiRequest(requestId, groupId, sort);
    requests.value = requests.value.map(existing => (existing.id === requestId ? request : existing));
    return request;
  }

  function updateDraftName(name: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.name = name;
  }

  function updateDraftMethod(method: ApiClientHttpMethod): void {
    if (!draft.value) {
      return;
    }
    draft.value.method = method;
  }

  function updateDraftUrl(rawUrl: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.url = rawUrl;
  }

  function pasteDraftUrl(rawUrl: string): void {
    if (!draft.value) {
      return;
    }
    const { baseUrl, rows } = splitUrlIntoBaseAndParams(rawUrl);
    draft.value.url = baseUrl;
    if (rows.length > 0) {
      draft.value.query = [...draft.value.query, ...rows];
    }
  }

  function updateDraftQuery(rows: ApiClientKeyValueRow[]): void {
    if (!draft.value) {
      return;
    }
    draft.value.query = rows;
  }

  function updateDraftHeaders(rows: ApiClientKeyValueRow[]): void {
    if (!draft.value) {
      return;
    }
    draft.value.headers = rows;
  }

  function updateDraftBody(body: ApiClientRequestBody): void {
    if (!draft.value) {
      return;
    }
    draft.value.body = body;
  }

  function updateDraftTimeoutMs(timeoutMs: number): void {
    if (!draft.value) {
      return;
    }
    draft.value.timeoutMs = Math.max(0, Math.round(timeoutMs));
  }

  function updateDraftAiPrompt(value: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.aiPrompt = value;
  }

  function updateDraftAiReference(value: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.aiReference = value;
  }

  function addQueryRow(): void {
    if (!draft.value) {
      return;
    }
    draft.value.query = [...draft.value.query, createEmptyRow()];
  }

  function addHeaderRow(): void {
    if (!draft.value) {
      return;
    }
    draft.value.headers = [...draft.value.headers, createEmptyRow()];
  }

  function addFormRow(): void {
    if (!draft.value) {
      return;
    }
    draft.value.body = {
      ...draft.value.body,
      form: [...draft.value.body.form, createEmptyRow()],
    };
  }

  function removeQueryRow(rowId: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.query = draft.value.query.filter(row => row.id !== rowId);
  }

  function removeHeaderRow(rowId: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.headers = draft.value.headers.filter(row => row.id !== rowId);
  }

  function removeFormRow(rowId: string): void {
    if (!draft.value) {
      return;
    }
    draft.value.body = {
      ...draft.value.body,
      form: draft.value.body.form.filter(row => row.id !== rowId),
    };
  }

  function selectEnvironment(environmentId: string | null): void {
    activeEnvironmentId.value = environmentId;
    clearExecutionState();
  }

  function discardDraftChanges(): void {
    if (!draft.value || !lastSavedRequestSnapshot.value) {
      return;
    }
    const saved = lastSavedRequestSnapshot.value;
    const savedName = draft.value.savedName;
    draft.value = {
      ...draft.value,
      name: savedName,
      method: saved.method,
      url: saved.url,
      query: cloneRows(saved.query),
      headers: cloneRows(saved.headers),
      body: cloneBody(saved.body),
      timeoutMs: saved.timeoutMs,
    };
  }

  function clearExecutionState(): void {
    executionAbortController.value?.abort();
    executionAbortController.value = null;
    execution.value = null;
  }

  function clearAiState(): void {
    aiAbortController.value?.abort();
    aiAbortController.value = null;
    aiCandidate.value = null;
    isGeneratingAi.value = false;
  }

  async function executeRequest(): Promise<void> {
    if (!draft.value || !activeRequestId.value || execution.value?.isRunning) {
      return;
    }

    const executionId = createId('exec');
    const controller = new AbortController();
    executionAbortController.value = controller;
    const env = activeEnvironment.value;
    const environmentId = env?.id ?? null;

    const snapshot = buildRequestSnapshot({
      method: draft.value.method,
      url: draft.value.url,
      query: draft.value.query,
      headers: draft.value.headers,
      body: draft.value.body,
      timeoutMs: draft.value.timeoutMs,
      environmentName: env?.name ?? null,
    });

    const lookup = env ? buildEnvironmentLookup(env.variables, env.isSensitive) : null;
    const urlResolution = resolveExecutionUrl({
      baseUrl: env?.baseUrl ?? '',
      url: draft.value.url,
      query: draft.value.query,
      headers: draft.value.headers,
      body: draft.value.body,
      environment: env ? { baseUrl: env.baseUrl, name: env.name } : null,
      lookup,
    });

    const headerMissing = lookup ? collectMissingVariablesInRows(draft.value.headers, lookup) : [];
    const bodyMissing = lookup ? collectMissingVariablesInBody(draft.value.body, lookup) : [];

    const allMissing = Array.from(new Set([
      ...urlResolution.missingVariables,
      ...headerMissing,
      ...bodyMissing,
    ]));

    if (allMissing.length > 0) {
      execution.value = {
        executionId,
        requestId: activeRequestId.value,
        isRunning: false,
        result: null,
        responseView: buildResponseView({
          kind: 'idle',
          rawText: '',
          meta: null,
          errorMessage: `存在未定义变量: ${allMissing.join(', ')}`,
        }),
      };
      throw new Error(`存在未定义变量: ${allMissing.join(', ')}`);
    }

    execution.value = {
      executionId,
      requestId: activeRequestId.value,
      isRunning: true,
      result: null,
      responseView: buildResponseView({
        kind: 'loading',
        rawText: '',
        meta: null,
        errorMessage: null,
      }),
    };

    try {
      const result = await service.executeApiRequest({
        requestId: activeRequestId.value,
        executionId,
        snapshot,
        environmentId,
      });

      if (execution.value?.executionId !== executionId || controller.signal.aborted) {
        return;
      }

      const kind = classifyResponseKind({
        cancelled: result.cancelled,
        timedOut: result.timedOut,
        connectionFailed: result.connectionFailed,
        statusCode: result.statusCode,
        isOversized: result.oversize,
        isBinary: result.isBinary,
      });

      const maskedHeaders = maskHeadersForHistory(result.responseHeaders);

      execution.value = {
        executionId,
        requestId: activeRequestId.value,
        isRunning: false,
        result: {
          ...result,
          responseHeaders: maskedHeaders,
        },
        responseView: buildResponseView({
          kind,
          rawText: result.responseBody,
          meta: {
            status: result.statusCode ?? 0,
            durationMs: result.durationMs,
            sizeBytes: result.sizeBytes,
            contentType: result.contentType,
            headers: maskedHeaders,
          },
          errorMessage: result.errorMessage,
          isOversized: result.oversize,
          isBinary: result.isBinary,
          isJsonTruncated: result.responseTruncated,
        }),
      };

      await loadHistory(activeRequestId.value);
    } catch (error) {
      if (execution.value?.executionId !== executionId || controller.signal.aborted) {
        return;
      }
      const message = error instanceof Error ? error.message : '请求失败';
      execution.value = {
        executionId,
        requestId: activeRequestId.value,
        isRunning: false,
        result: null,
        responseView: buildResponseView({
          kind: 'network_error',
          rawText: '',
          meta: null,
          errorMessage: message,
        }),
      };
    } finally {
      if (execution.value?.executionId === executionId) {
        executionAbortController.value = null;
      }
    }
  }

  async function cancelExecution(): Promise<void> {
    const current = execution.value;
    if (!current || !current.isRunning) {
      return;
    }
    executionAbortController.value?.abort();
    try {
      await service.cancelApiExecution(current.executionId);
    } catch {
      // ignore: cancellation is best-effort
    }
    execution.value = {
      ...current,
      isRunning: false,
      responseView: buildResponseView({
        kind: 'cancelled',
        rawText: current.responseView.rawText,
        meta: current.responseView.meta,
        errorMessage: '已取消请求',
      }),
    };
    executionAbortController.value = null;
  }

  async function loadHistory(requestId: string): Promise<void> {
    isLoadingHistory.value = true;
    try {
      history.value = await service.listApiRequestHistory(requestId);
    } finally {
      isLoadingHistory.value = false;
    }
  }

  async function clearHistory(): Promise<void> {
    if (!activeRequestId.value) {
      return;
    }
    isClearingHistory.value = true;
    try {
      await service.clearApiRequestHistory(activeRequestId.value);
      history.value = [];
    } finally {
      isClearingHistory.value = false;
    }
  }

  function applyHistoryToDraft(historyId: string): void {
    if (!draft.value) {
      return;
    }
    const entry = history.value.find(item => item.id === historyId);
    if (!entry) {
      return;
    }
    const snapshot = entry.requestSnapshot;
    draft.value = {
      ...draft.value,
      method: snapshot.method,
      url: snapshot.url,
      query: cloneRows(snapshot.query),
      headers: cloneRows(snapshot.headers),
      body: cloneBody(snapshot.body),
      timeoutMs: snapshot.timeoutMs,
    };
    execution.value = null;
  }

  async function loadAiModels(): Promise<void> {
    try {
      aiAvailableModels.value = await service.listApiAiModels();
    } catch {
      aiAvailableModels.value = [];
    }
  }

  async function generateAiBody(options: { prompt: string; reference: string; includeCurrentBody: boolean }): Promise<void> {
    if (!draft.value || !activeRequestId.value) {
      return;
    }
    if (isGeneratingAi.value) {
      return;
    }

    const taskId = createId('ai');
    const controller = new AbortController();
    aiAbortController.value = controller;

    const modelOption = aiAvailableModels.value.find(option => `${option.providerId}/${option.modelId}` === aiSelectedModelKey.value)
      ?? aiAvailableModels.value[0]
      ?? null;

    if (!modelOption) {
      aiCandidate.value = buildAiCandidateFailed({
        requestId: activeRequestId.value,
        taskId,
        prompt: options.prompt,
        reference: options.reference,
        includeCurrentBody: options.includeCurrentBody,
        modelLabel: '',
        errorMessage: '请先在设置中配置模型',
      });
      throw new Error('未配置可用模型');
    }

    const modelLabel = modelOption.label;
    aiCandidate.value = buildAiCandidateGenerating({
      requestId: activeRequestId.value,
      taskId,
      prompt: options.prompt,
      reference: options.reference,
      includeCurrentBody: options.includeCurrentBody,
      modelLabel,
    });
    isGeneratingAi.value = true;

    try {
      const payload = buildAiContextPayload({
        prompt: options.prompt,
        reference: options.reference,
        includeCurrentBody: options.includeCurrentBody,
        currentBody: draft.value.body,
        requestName: draft.value.name,
        method: draft.value.method,
        url: draft.value.url,
      });

      const result = await service.generateApiBody({
        requestId: activeRequestId.value,
        taskId,
        prompt: payload.prompt,
        reference: payload.reference,
        includeCurrentBody: payload.hasCurrentBody,
        currentBody: payload.hasCurrentBody ? draft.value.body : null,
        requestName: payload.requestName,
        method: payload.method,
        url: payload.url,
      });

      if (aiCandidate.value?.taskId !== taskId || controller.signal.aborted) {
        return;
      }

      aiCandidate.value = buildAiCandidate({
        requestId: activeRequestId.value,
        taskId,
        prompt: options.prompt,
        reference: options.reference,
        includeCurrentBody: options.includeCurrentBody,
        modelLabel: result.model_label || modelLabel,
        rawContent: result.content,
      });
    } catch (error) {
      if (aiCandidate.value?.taskId !== taskId || controller.signal.aborted) {
        return;
      }
      const message = error instanceof Error ? error.message : '生成失败';
      aiCandidate.value = buildAiCandidateFailed({
        requestId: activeRequestId.value,
        taskId,
        prompt: options.prompt,
        reference: options.reference,
        includeCurrentBody: options.includeCurrentBody,
        modelLabel,
        errorMessage: message,
      });
    } finally {
      if (aiCandidate.value?.taskId === taskId) {
        isGeneratingAi.value = false;
        aiAbortController.value = null;
      }
    }
  }

  async function cancelAiGeneration(): Promise<void> {
    const candidate = aiCandidate.value;
    if (!candidate || candidate.status !== 'generating') {
      return;
    }
    aiAbortController.value?.abort();
    try {
      await service.cancelApiGeneration(candidate.taskId);
    } catch {
      // ignore: cancellation is best-effort
    }
    aiCandidate.value = {
      ...candidate,
      status: 'cancelled',
    };
    isGeneratingAi.value = false;
    aiAbortController.value = null;
  }

  function applyAiCandidateToBody(): boolean {
    if (!draft.value || !aiCandidate.value) {
      return false;
    }
    const candidate = aiCandidate.value;
    if (!candidate.isJsonValid || candidate.status !== 'success') {
      return false;
    }
    draft.value.body = applyCandidateToBody(draft.value.body, candidate);
    return true;
  }

  function setSearchKeyword(keyword: string): void {
    requestListFilter.value = keyword;
  }

  function selectAiModel(key: string | null): void {
    aiSelectedModelKey.value = key;
  }

  function editAiCandidateContent(value: string): void {
    if (!aiCandidate.value || aiCandidate.value.status !== 'success') {
      return;
    }
    const validation = tryFormatJson(value);
    aiCandidate.value = {
      ...aiCandidate.value,
      content: value,
      isJsonValid: validation.ok,
      jsonError: validation.error,
    };
  }

  return {
    projects,
    groups,
    requests,
    environments,
    activeProjectId,
    activeGroupId,
    activeRequestId,
    activeEnvironmentId,
    draft,
    lastSavedRequestSnapshot,
    isLoadingProjects,
    isLoadingGroups,
    isLoadingRequests,
    isLoadingEnvironments,
    isSavingRequest,
    isDeletingProject,
    isDeletingGroup,
    isDeletingRequest,
    execution,
    aiCandidate,
    aiAvailableModels,
    aiSelectedModelKey,
    isGeneratingAi,
    history,
    isLoadingHistory,
    isClearingHistory,
    requestListFilter,
    searchKeyword,
    activeProject,
    activeGroup,
    activeEnvironment,
    activeRequest,
    requestSearchResults,
    isDirty,
    loadProjects,
    loadGroups,
    loadRequests,
    loadEnvironments,
    selectProject,
    createProject,
    renameProject,
    updateProject,
    deleteProject,
    createGroup,
    renameGroup,
    deleteGroup,
    selectRequest,
    createRequest,
    saveDraft,
    duplicateRequest,
    deleteRequest,
    moveRequest,
    updateDraftName,
    updateDraftMethod,
    updateDraftUrl,
    pasteDraftUrl,
    updateDraftQuery,
    updateDraftHeaders,
    updateDraftBody,
    updateDraftTimeoutMs,
    updateDraftAiPrompt,
    updateDraftAiReference,
    addQueryRow,
    addHeaderRow,
    addFormRow,
    removeQueryRow,
    removeHeaderRow,
    removeFormRow,
    selectEnvironment,
    discardDraftChanges,
    executeRequest,
    cancelExecution,
    loadHistory,
    clearHistory,
    applyHistoryToDraft,
    loadAiModels,
    generateAiBody,
    cancelAiGeneration,
    applyAiCandidateToBody,
    setSearchKeyword,
    selectAiModel,
    editAiCandidateContent,
  };
}

function buildDraftFromRequest(request: ApiClientRequest): RequestDraft {
  return {
    id: request.id,
    name: request.name,
    method: request.method,
    url: request.url,
    query: cloneRows(request.query),
    headers: cloneRows(request.headers),
    body: cloneBody(request.body),
    timeoutMs: request.timeoutMs,
    aiPrompt: request.aiPrompt,
    aiReference: request.aiReference,
    savedName: request.name,
  };
}

export const useApiClientStore = defineStore('api-client', () => createApiClientStore());

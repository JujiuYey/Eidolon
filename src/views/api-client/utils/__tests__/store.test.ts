import { strict as assert } from 'node:assert';
import { afterEach, beforeEach, describe, it } from 'vitest';
import { createPinia, defineStore, setActivePinia } from 'pinia';
import { createApiClientStore } from '@/stores/api-client';
import type { ApiClientService } from '@/stores/api-client';
import type {
  ApiClientEnvironment,
  ApiClientGroup,
  ApiClientProject,
  ApiClientRequest,
  ApiClientRequestHistory,
  ApiClientRequestSnapshot,
  ApiSendRequestInput,
} from '@/types/api-client';

function bindStore(service: ApiClientService) {
  const useBound = defineStore('api-client-test', () => createApiClientStore({ service }));
  return useBound();
}

interface RecordedCall {
  method: string;
  args: unknown[];
}

function makeRow(overrides: Partial<{ id: string; enabled: boolean; key: string; value: string }> = {}) {
  return {
    id: overrides.id ?? `row_${Math.random().toString(36).slice(2, 10)}`,
    enabled: overrides.enabled ?? true,
    key: overrides.key ?? '',
    value: overrides.value ?? '',
  };
}

function makeProject(overrides: Partial<ApiClientProject> = {}): ApiClientProject {
  return {
    id: 'project-1',
    name: 'Sample',
    description: '',
    sort: 0,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

function makeGroup(overrides: Partial<ApiClientGroup> = {}): ApiClientGroup {
  return {
    id: 'group-1',
    projectId: 'project-1',
    parentGroupId: null,
    name: 'Default',
    sort: 0,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

function makeEnvironment(overrides: Partial<ApiClientEnvironment> = {}): ApiClientEnvironment {
  return {
    id: 'env-1',
    projectId: 'project-1',
    name: 'dev',
    baseUrl: 'https://api.example.com',
    variables: [makeRow({ key: 'host', value: 'https://api.example.com' })],
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

function makeRequest(overrides: Partial<ApiClientRequest> = {}): ApiClientRequest {
  return {
    id: 'request-1',
    groupId: 'group-1',
    projectId: 'project-1',
    name: 'list users',
    method: 'GET',
    url: 'https://api.example.com/users',
    query: [],
    headers: [],
    body: { kind: 'none', text: '', form: [] },
    timeoutMs: 5000,
    aiPrompt: '',
    aiReference: '',
    sort: 0,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

interface ExecuteInput {
  requestId: string;
  executionId: string;
  snapshot: ApiSendRequestInput['snapshot'];
  environmentId: string | null;
}

interface SnakeExecutionResult {
  execution_id: string;
  status: 'success' | 'http_error' | 'network_error' | 'timeout' | 'cancelled' | 'oversize';
  status_code: number | null;
  duration_ms: number;
  size_bytes: number;
  content_type: string | null;
  response_headers: Array<{ id: string; enabled: boolean; key: string; value: string }>;
  response_body: string;
  response_truncated: boolean;
  is_binary: boolean;
  oversize: boolean;
  connection_failed: boolean;
  timed_out: boolean;
  cancelled: boolean;
  error_message: string | null;
  history_id: string | null;
  history_error?: string | null;
}

interface GenerateInput {
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  currentBody: string | null;
  requestName: string;
  method: string;
  url: string;
}

interface SnakeAiGenerateResult {
  content: string;
  is_json_valid: boolean;
  json_error: string | null;
  model_label: string;
}

interface ServiceState {
  service: ApiClientService;
  calls: RecordedCall[];
  setExecuteImpl: (impl: (input: ExecuteInput) => Promise<SnakeExecutionResult>) => void;
  getExecuteImpl: () => (input: ExecuteInput) => Promise<SnakeExecutionResult>;
  setGenerateImpl: (impl: (input: GenerateInput) => Promise<SnakeAiGenerateResult>) => void;
  getGenerateImpl: () => (input: GenerateInput) => Promise<SnakeAiGenerateResult>;
  historyFixture: ApiClientRequestHistory[];
}

function buildService(): ServiceState {
  const calls: RecordedCall[] = [];
  let executeImpl: (input: ExecuteInput) => Promise<SnakeExecutionResult>
    = async () => {
      throw new Error('executeImpl not configured');
    };
  let generateImpl: (input: GenerateInput) => Promise<SnakeAiGenerateResult>
    = async () => {
      throw new Error('generateImpl not configured');
    };
  const historyFixture: ApiClientRequestHistory[] = [];

  const service: ApiClientService = {
    listApiProjects: async () => {
      calls.push({ method: 'listApiProjects', args: [] });
      return [makeProject()];
    },
    createApiProject: async (name: string, description: string) => {
      calls.push({ method: 'createApiProject', args: [name, description] });
      return makeProject({ name, description });
    },
    renameApiProject: async (projectId: string, name: string) => {
      calls.push({ method: 'renameApiProject', args: [projectId, name] });
      return makeProject({ id: projectId, name });
    },
    updateApiProject: async (projectId: string, name: string, description: string) => {
      calls.push({ method: 'updateApiProject', args: [projectId, name, description] });
      return makeProject({ id: projectId, name, description });
    },
    previewApiProjectDeletion: async (projectId: string) => {
      calls.push({ method: 'previewApiProjectDeletion', args: [projectId] });
      return { deletedRequests: 0, deletedHistories: 0 };
    },
    deleteApiProject: async (projectId: string) => {
      calls.push({ method: 'deleteApiProject', args: [projectId] });
      return { deletedRequests: 0, deletedHistories: 0 };
    },
    listApiGroups: async (projectId: string) => {
      calls.push({ method: 'listApiGroups', args: [projectId] });
      return [makeGroup({ projectId })];
    },
    createApiGroup: async (projectId: string, name: string) => {
      calls.push({ method: 'createApiGroup', args: [projectId, name] });
      return makeGroup({ projectId, name });
    },
    renameApiGroup: async (groupId: string, name: string) => {
      calls.push({ method: 'renameApiGroup', args: [groupId, name] });
      return makeGroup({ id: groupId, name });
    },
    deleteApiGroup: async (groupId: string) => {
      calls.push({ method: 'deleteApiGroup', args: [groupId] });
      return { deletedRequests: 0, deletedHistories: 0 };
    },
    reorderApiGroups: async (projectId: string, groupIds: string[]) => {
      calls.push({ method: 'reorderApiGroups', args: [projectId, groupIds] });
      return [];
    },
    listApiRequests: async (projectId: string) => {
      calls.push({ method: 'listApiRequests', args: [projectId] });
      return [makeRequest({ projectId })];
    },
    getApiRequest: async (requestId: string) => {
      calls.push({ method: 'getApiRequest', args: [requestId] });
      if (requestId === 'missing') {
        return null;
      }
      return makeRequest({ id: requestId });
    },
    createApiRequest: async (input: { groupId: string; name: string }) => {
      calls.push({ method: 'createApiRequest', args: [input] });
      return makeRequest({
        id: 'request-new',
        groupId: input.groupId,
        name: input.name,
      });
    },
    updateApiRequest: async (request: ApiClientRequest) => {
      calls.push({ method: 'updateApiRequest', args: [request] });
      return request;
    },
    duplicateApiRequest: async (requestId: string) => {
      calls.push({ method: 'duplicateApiRequest', args: [requestId] });
      return makeRequest({ id: `${requestId}-copy`, name: `Copy of ${requestId}` });
    },
    moveApiRequest: async (requestId: string, targetGroupId: string) => {
      calls.push({ method: 'moveApiRequest', args: [requestId, targetGroupId] });
      return makeRequest({ id: requestId, groupId: targetGroupId });
    },
    deleteApiRequest: async (requestId: string) => {
      calls.push({ method: 'deleteApiRequest', args: [requestId] });
      return { deletedHistories: 0 };
    },
    reorderApiRequests: async (groupId: string, requestIds: string[]) => {
      calls.push({ method: 'reorderApiRequests', args: [groupId, requestIds] });
      return [];
    },
    listApiEnvironments: async (projectId: string) => {
      calls.push({ method: 'listApiEnvironments', args: [projectId] });
      return [makeEnvironment({ projectId })];
    },
    upsertApiEnvironment: async (env: ApiClientEnvironment) => {
      calls.push({ method: 'upsertApiEnvironment', args: [env] });
      return env;
    },
    deleteApiEnvironment: async (environmentId: string) => {
      calls.push({ method: 'deleteApiEnvironment', args: [environmentId] });
    },
    sendApiRequest: async (input: ExecuteInput) => {
      calls.push({ method: 'sendApiRequest', args: [input] });
      const result = await executeImpl(input);
      return {
        executionId: result.execution_id,
        status: result.status,
        statusCode: result.status_code,
        durationMs: result.duration_ms,
        sizeBytes: result.size_bytes,
        contentType: result.content_type,
        responseHeaders: result.response_headers,
        responseBody: result.response_body,
        responseTruncated: result.response_truncated,
        isBinary: result.is_binary,
        oversize: result.oversize,
        connectionFailed: result.connection_failed,
        timedOut: result.timed_out,
        cancelled: result.cancelled,
        errorMessage: result.error_message,
        historyId: result.history_id,
        historyError: result.history_error ?? null,
      };
    },
    cancelApiRequest: async (executionId: string) => {
      calls.push({ method: 'cancelApiRequest', args: [executionId] });
      return true;
    },
    previewApiRequest: async (input: { requestId: string; environmentId: string | null; snapshot: ApiSendRequestInput['snapshot'] }) => {
      calls.push({ method: 'previewApiRequest', args: [input] });
      return { method: input.snapshot.method, url: input.snapshot.url, bodySizeBytes: 0, environmentName: null };
    },
    listApiRequestHistories: async (requestId: string) => {
      calls.push({ method: 'listApiRequestHistories', args: [requestId] });
      return [...historyFixture];
    },
    getApiRequestHistory: async (historyId: string) => {
      calls.push({ method: 'getApiRequestHistory', args: [historyId] });
      return historyFixture.find(item => item.id === historyId) ?? null;
    },
    clearApiRequestHistories: async (requestId: string) => {
      calls.push({ method: 'clearApiRequestHistories', args: [requestId] });
      return 0;
    },
    generateApiRequestBody: async (input: GenerateInput) => {
      calls.push({ method: 'generateApiRequestBody', args: [input] });
      const result = await generateImpl(input);
      return {
        content: result.content,
        isJsonValid: result.is_json_valid,
        jsonError: result.json_error,
        modelLabel: result.model_label,
      };
    },
  };

  return {
    service,
    calls,
    getExecuteImpl: () => executeImpl,
    setExecuteImpl: (value: typeof executeImpl) => {
      executeImpl = value;
    },
    getGenerateImpl: () => generateImpl,
    setGenerateImpl: (value: typeof generateImpl) => {
      generateImpl = value;
    },
    historyFixture,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
});

afterEach(() => {
  setActivePinia(null as unknown as ReturnType<typeof createPinia>);
});

describe('selectProject bootstraps groups, requests, environments', () => {
  it('loads project + child collections and selects first environment', async () => {
    const state = buildService();
    const store = bindStore(state.service);

    await store.loadProjects();
    await store.selectProject('project-1', { discardUnsaved: true });

    assert.equal(store.activeProjectId, 'project-1');
    assert.equal(store.activeEnvironmentId, 'env-1');
    assert.equal(store.groups.length, 1);
    assert.equal(store.requests.length, 1);
    assert.equal(store.environments.length, 1);

    const methods = state.calls.map(call => call.method);
    assert.ok(methods.includes('listApiGroups'));
    assert.ok(methods.includes('listApiRequests'));
    assert.ok(methods.includes('listApiEnvironments'));
  });
});

describe('empty initial state', () => {
  it('starts with no draft, no active request, and no dirty flag', async () => {
    const state = buildService();
    const store = bindStore(state.service);

    assert.equal(store.draft, null);
    assert.equal(store.activeRequestId, null);
    assert.equal(store.isDirty, false);
  });
});

describe('updateProject persists name and description via update_api_project', () => {
  it('calls updateApiProject with the right camelCase args and replaces the project in store', async () => {
    const state = buildService();
    const store = bindStore(state.service);
    await store.loadProjects();
    const initial = store.projects[0];
    assert.ok(initial);

    const updated = await store.updateProject(initial.id, '新名称', '新描述');

    const updateCalls = state.calls.filter(call => call.method === 'updateApiProject');
    assert.equal(updateCalls.length, 1);
    assert.deepEqual(updateCalls[0]!.args, [initial.id, '新名称', '新描述']);

    const renameCalls = state.calls.filter(call => call.method === 'renameApiProject');
    assert.equal(renameCalls.length, 0, 'must not fall back to rename + local patch');

    assert.equal(updated.name, '新名称');
    assert.equal(updated.description, '新描述');

    const persisted = store.projects.find(item => item.id === initial.id);
    assert.ok(persisted);
    assert.equal(persisted.name, '新名称');
    assert.equal(persisted.description, '新描述');
  });

  it('surfaces backend error when update_api_project fails', async () => {
    const state = buildService();
    state.service.updateApiProject = async () => {
      throw new Error('name 重复');
    };
    const store = bindStore(state.service);
    await store.loadProjects();
    const initial = store.projects[0];
    assert.ok(initial);

    await assert.rejects(() => store.updateProject(initial.id, '名称', '描述'), /name 重复/);

    const persisted = store.projects.find(item => item.id === initial.id);
    assert.ok(persisted);
    assert.notEqual(persisted.name, '名称', 'local state must not be mutated when the command fails');
  });
});

describe('uRL Query parsing on paste', () => {
  it('splits URL into base + repeated query rows enabled by default', async () => {
    const state = buildService();
    const store = bindStore(state.service);

    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    store.pasteDraftUrl('https://api.example.com/users?tag=a&tag=b&token=secret');

    assert.equal(store.draft?.url, 'https://api.example.com/users');
    assert.equal(store.draft?.query.length, 3);
    assert.deepEqual(
      store.draft?.query.map(row => [row.key, row.value, row.enabled]),
      [['tag', 'a', true], ['tag', 'b', true], ['token', 'secret', true]],
    );
  });
});

describe('saveDraft vs send isolation', () => {
  it('executeRequest does not call updateApiRequest and does not mutate saved snapshot', async () => {
    const state = buildService();
    state.setExecuteImpl(async input => {
      return {
        execution_id: input.executionId,
        status: 'success',
        status_code: 200,
        duration_ms: 12,
        size_bytes: 2,
        content_type: 'application/json',
        response_headers: [],
        response_body: '{"ok":true}',
        response_truncated: false,
        is_binary: false,
        oversize: false,
        connection_failed: false,
        timed_out: false,
        cancelled: false,
        error_message: null,
        history_id: 'history-1',
      };
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftUrl('https://api.example.com/users-modified');
    store.updateDraftQuery([makeRow({ key: 'limit', value: '5', enabled: true })]);

    await store.executeRequest();

    assert.equal(store.execution?.responseView.kind, 'success');
    assert.equal(store.lastSavedRequestSnapshot?.url, 'https://api.example.com/users');

    const saveCalls = state.calls.filter(call => call.method === 'updateApiRequest');
    assert.equal(saveCalls.length, 0, 'executeRequest must not persist the draft');

    const listCalls = state.calls.filter(call => call.method === 'listApiRequestHistories');
    assert.ok(listCalls.length > 0, 'executeRequest should refresh history');
  });
});

describe('executeRequest missing-variable guard', () => {
  it('does not invoke the network when an unresolved variable is present', async () => {
    const state = buildService();
    const store = bindStore(state.service);

    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftUrl('https://{{undefined_host}}/users');

    await assert.rejects(() => store.executeRequest(), /未定义变量/);

    const executeCalls = state.calls.filter(call => call.method === 'sendApiRequest');
    assert.equal(executeCalls.length, 0, 'executeRequest must block before invoke');
    assert.equal(store.execution?.responseView.kind, 'idle');
    assert.match(store.execution?.responseView.errorMessage ?? '', /未定义变量/);
  });
});

describe('executeRequest race ID handling', () => {
  it('discards late response whose executionId no longer matches the current execution', async () => {
    const state = buildService();
    const pendingFirsts: Array<{ resolve: (value: SnakeExecutionResult) => void; executionId: string }> = [];
    state.setExecuteImpl(async input => {
      return new Promise<SnakeExecutionResult>(resolve => {
        pendingFirsts.push({ resolve, executionId: input.executionId });
      });
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    const firstPromise = store.executeRequest();
    await new Promise(resolve => setTimeout(resolve, 0));
    const firstEntry = pendingFirsts.shift();
    assert.ok(firstEntry);
    const firstExecutionId = firstEntry.executionId;
    assert.equal(store.execution?.executionId, firstExecutionId);

    firstEntry.resolve({
      execution_id: firstExecutionId,
      status: 'success',
      status_code: 200,
      duration_ms: 999,
      size_bytes: 0,
      content_type: null,
      response_headers: [],
      response_body: '{"first":true}',
      response_truncated: false,
      is_binary: false,
      oversize: false,
      connection_failed: false,
      timed_out: false,
      cancelled: false,
      error_message: null,
      history_id: null,
    });
    await firstPromise;
    assert.equal(store.execution?.responseView.rawText, '{"first":true}');

    const secondPromise = store.executeRequest();
    await new Promise(resolve => setTimeout(resolve, 0));
    const secondEntry = pendingFirsts.shift();
    assert.ok(secondEntry);
    const secondExecutionId = secondEntry.executionId;
    assert.notEqual(secondExecutionId, firstExecutionId);

    secondEntry.resolve({
      execution_id: secondExecutionId,
      status: 'success',
      status_code: 200,
      duration_ms: 5,
      size_bytes: 0,
      content_type: null,
      response_headers: [],
      response_body: '{"latest":true}',
      response_truncated: false,
      is_binary: false,
      oversize: false,
      connection_failed: false,
      timed_out: false,
      cancelled: false,
      error_message: null,
      history_id: 'history-latest',
    });

    await secondPromise;

    assert.notEqual(store.execution?.executionId, firstExecutionId);
    assert.equal(store.execution?.responseView.rawText, '{"latest":true}');
  });
});

describe('applyHistoryToDraft never auto-sends', () => {
  it('updates draft but does not trigger executeApiRequest', async () => {
    const state = buildService();
    state.historyFixture.push({
      id: 'history-1',
      requestId: 'request-1',
      environmentName: 'dev',
      requestSnapshot: {
        method: 'POST',
        url: 'https://api.example.com/orders',
        query: [],
        headers: [],
        body: { kind: 'json', text: '{"orderId":"o-1"}', form: [] },
        timeoutMs: 5000,
        environmentName: 'dev',
      },
      status: 'success',
      statusCode: 201,
      responseHeaders: [],
      responseBodyPreview: '{"orderId":"o-1"}',
      responseBodyTruncated: false,
      durationMs: 32,
      errorMessage: null,
      executedAt: 1,
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    store.applyHistoryToDraft('history-1');

    assert.equal(store.draft?.method, 'POST');
    assert.equal(store.draft?.url, 'https://api.example.com/orders');
    assert.equal(store.draft?.body.kind, 'json');
    assert.equal(store.draft?.body.text, '{"orderId":"o-1"}');
    assert.equal(store.execution, null, 'restoring must not run a new execution');

    const executeCalls = state.calls.filter(call => call.method === 'sendApiRequest');
    assert.equal(executeCalls.length, 0);
  });
});

describe('aI candidate does not auto-apply', () => {
  it('returns content as candidate and leaves draft body untouched', async () => {
    const state = buildService();
    state.setGenerateImpl(async () => {
      return {
        content: '{"orderId":"o-1","items":[]}',
        is_json_valid: true,
        json_error: null,
        model_label: 'mock-model',
      };
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftBody({ kind: 'json', text: '{"old":true}', form: [] });

    await store.generateAiBody({
      prompt: '生成订单',
      reference: '',
      includeCurrentBody: false,
    });

    assert.equal(store.aiCandidate?.status, 'success');
    assert.equal(store.aiCandidate?.isJsonValid, true);
    assert.equal(store.aiCandidate?.content, '{"orderId":"o-1","items":[]}');
    assert.equal(store.draft?.body.text, '{"old":true}', 'draft body must not be modified automatically');

    const executedAfterGen = state.calls.filter(call => call.method === 'sendApiRequest');
    assert.equal(executedAfterGen.length, 0, 'AI generation must not trigger execution');
  });
});

describe('aI generation race ID handling', () => {
  it('drops late result when a newer task is started', async () => {
    const state = buildService();
    const pending: Array<{ resolve: (value: SnakeAiGenerateResult) => void; taskId: string }> = [];
    state.setGenerateImpl(async () => {
      return new Promise<SnakeAiGenerateResult>(resolve => {
        pending.push({ resolve, taskId: `task-${Math.random().toString(36).slice(2, 10)}` });
      });
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    const firstPromise = store.generateAiBody({ prompt: 'p', reference: '', includeCurrentBody: false });
    await new Promise(resolve => setTimeout(resolve, 0));
    const firstEntry = pending.shift();
    assert.ok(firstEntry);
    const firstStoreTaskId = store.aiCandidate?.taskId;
    assert.ok(firstStoreTaskId);

    firstEntry.resolve({
      content: '{"first":true}',
      is_json_valid: true,
      json_error: null,
      model_label: 'mock-model',
    });
    await firstPromise;
    assert.equal(store.aiCandidate?.content, '{"first":true}');
    const completedFirstTaskId = store.aiCandidate?.taskId;

    const secondPromise = store.generateAiBody({ prompt: 'p', reference: '', includeCurrentBody: false });
    await new Promise(resolve => setTimeout(resolve, 0));
    const secondEntry = pending.shift();
    assert.ok(secondEntry);
    const secondStoreTaskId = store.aiCandidate?.taskId;
    assert.ok(secondStoreTaskId);
    assert.notEqual(secondStoreTaskId, completedFirstTaskId);

    secondEntry.resolve({
      content: '{"latest":true}',
      is_json_valid: true,
      json_error: null,
      model_label: 'mock-model',
    });

    await secondPromise;

    assert.notEqual(store.aiCandidate?.taskId, completedFirstTaskId);
    assert.equal(store.aiCandidate?.content, '{"latest":true}');
  });
});

describe('selectRequest refuses when dirty without explicit discard', () => {
  it('throws and keeps the active draft when not discarding', async () => {
    const state = buildService();
    const store = bindStore(state.service);

    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftUrl('https://api.example.com/mutated');

    await assert.rejects(() => store.selectRequest('request-2'), /未保存/);
    assert.equal(store.activeRequestId, 'request-1');
  });

  it('switches request when discardUnsaved is true', async () => {
    const state = buildService();
    const originalList = state.service.listApiRequests;
    state.service.listApiRequests = async (projectId: string) => {
      const items = await originalList(projectId);
      items.push(makeRequest({ id: 'request-2', projectId }));
      return items;
    };

    const store = bindStore(state.service);

    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftUrl('https://api.example.com/mutated');

    await store.selectRequest('request-2', { discardUnsaved: true });
    assert.equal(store.activeRequestId, 'request-2');
  });
});

describe('selectGroup updates active group and clears request context', () => {
  it('updates activeGroupId and clears activeRequestId/draft/execution', async () => {
    const state = buildService();
    const originalList = state.service.listApiGroups;
    state.service.listApiGroups = async (projectId: string) => {
      const items = await originalList(projectId);
      items.push(makeGroup({ id: 'group-2', projectId }));
      return items;
    };

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    assert.equal(store.activeGroupId, 'group-1');
    assert.equal(store.activeRequestId, 'request-1');
    assert.ok(store.draft);

    store.selectGroup('group-2');

    assert.equal(store.activeGroupId, 'group-2');
    assert.equal(store.activeRequestId, null);
    assert.equal(store.draft, null);
    assert.equal(store.lastSavedRequestSnapshot, null);
    assert.equal(store.execution, null);
  });

  it('is a no-op when the group does not belong to the active project', async () => {
    const state = buildService();
    state.service.listApiGroups = async (projectId: string) => {
      return [makeGroup({ id: 'group-1', projectId }), makeGroup({ id: 'group-other', projectId: 'project-other' })];
    };

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    store.selectGroup('group-other');

    assert.equal(store.activeGroupId, 'group-1', 'must not leak active group id across projects');
    assert.equal(store.activeRequestId, 'request-1');
    assert.ok(store.draft);
  });

  it('clears running AI candidate state when switching groups', async () => {
    const state = buildService();
    state.setGenerateImpl(async () => ({
      content: '{"a":1}',
      is_json_valid: true,
      json_error: null,
      model_label: 'm',
    }));
    const originalList = state.service.listApiGroups;
    state.service.listApiGroups = async (projectId: string) => {
      const items = await originalList(projectId);
      items.push(makeGroup({ id: 'group-2', projectId }));
      return items;
    };

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    await store.generateAiBody({ prompt: 'p', reference: '', includeCurrentBody: false });
    assert.ok(store.aiCandidate);

    store.selectGroup('group-2');

    assert.equal(store.aiCandidate, null);
  });
});

describe('key/value row disabling', () => {
  it('disabled rows are excluded from the execution query string', async () => {
    const state = buildService();
    const snapshotHolder: { value: ApiClientRequestSnapshot | null } = { value: null };
    state.setExecuteImpl(async input => {
      snapshotHolder.value = input.snapshot;
      return {
        execution_id: input.executionId,
        status: 'success',
        status_code: 200,
        duration_ms: 1,
        size_bytes: 0,
        content_type: null,
        response_headers: [],
        response_body: '',
        response_truncated: false,
        is_binary: false,
        oversize: false,
        connection_failed: false,
        timed_out: false,
        cancelled: false,
        error_message: null,
        history_id: null,
      };
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });
    store.updateDraftQuery([
      makeRow({ key: 'include', value: '1', enabled: true }),
      makeRow({ key: 'skip', value: '2', enabled: false }),
    ]);

    await store.executeRequest();
    const snapshot = snapshotHolder.value;
    assert.ok(snapshot);
    if (!snapshot) {
      throw new Error('expected snapshot to be set');
    }
    assert.equal(snapshot.query.length, 2);
    assert.equal(snapshot.query[0]!.enabled, true);
    assert.equal(snapshot.query[1]!.enabled, false);
  });
});

describe('cancelExecution transitions running state to cancelled', () => {
  it('does not mutate the saved request after cancellation', async () => {
    const state = buildService();
    state.setExecuteImpl(async input => {
      await new Promise(resolve => setTimeout(resolve, 10));
      return {
        execution_id: input.executionId,
        status: 'success',
        status_code: 200,
        duration_ms: 10,
        size_bytes: 0,
        content_type: null,
        response_headers: [],
        response_body: '{}',
        response_truncated: false,
        is_binary: false,
        oversize: false,
        connection_failed: false,
        timed_out: false,
        cancelled: true,
        error_message: null,
        history_id: null,
      };
    });

    const store = bindStore(state.service);
    await store.selectProject('project-1', { discardUnsaved: true });
    await store.selectRequest('request-1', { discardUnsaved: true });

    const promise = store.executeRequest();
    await store.cancelExecution();
    await promise;

    assert.equal(store.execution?.responseView.kind, 'cancelled');
    const saveCalls = state.calls.filter(call => call.method === 'updateApiRequest');
    assert.equal(saveCalls.length, 0);
  });
});

import { invoke } from '@tauri-apps/api/core';
import type {
  ApiClientBodyKind,
  ApiClientEnvironment,
  ApiClientGroup,
  ApiClientHttpMethod,
  ApiClientKeyValueRow,
  ApiClientProject,
  ApiClientRequest,
  ApiClientRequestBody,
  ApiClientRequestHistory,
  ApiClientRequestSnapshot,
} from '@/types/api-client';

interface TauriKeyValueRow {
  id: string;
  enabled: boolean;
  key: string;
  value: string;
}

interface TauriRequestBody {
  kind: ApiClientBodyKind;
  text: string;
  form: TauriKeyValueRow[];
}

interface TauriRequestSnapshot {
  method: ApiClientHttpMethod;
  url: string;
  query: TauriKeyValueRow[];
  headers: TauriKeyValueRow[];
  body: TauriRequestBody;
  timeout_ms: number;
  environment_name: string | null;
}

interface TauriProject {
  id: string;
  name: string;
  description: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

interface TauriGroup {
  id: string;
  project_id: string;
  parent_group_id: string | null;
  name: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

interface TauriRequest {
  id: string;
  group_id: string;
  project_id: string;
  name: string;
  method: ApiClientHttpMethod;
  url: string;
  query: TauriKeyValueRow[];
  headers: TauriKeyValueRow[];
  body: TauriRequestBody;
  timeout_ms: number;
  ai_prompt: string;
  ai_reference: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

interface TauriEnvironment {
  id: string;
  project_id: string;
  name: string;
  base_url: string;
  variables: TauriKeyValueRow[];
  is_sensitive: boolean;
  created_at: number;
  updated_at: number;
}

interface TauriRequestHistory {
  id: string;
  request_id: string;
  project_id: string;
  environment_name: string | null;
  request_snapshot: TauriRequestSnapshot;
  status: string;
  status_code: number | null;
  response_headers: TauriKeyValueRow[];
  response_body_preview: string;
  response_body_truncated: boolean;
  duration_ms: number;
  error_message: string | null;
  executed_at: number;
}

function toFrontendRow(row: TauriKeyValueRow): ApiClientKeyValueRow {
  return {
    id: row.id,
    enabled: row.enabled,
    key: row.key,
    value: row.value,
  };
}

function toTauriRow(row: ApiClientKeyValueRow): TauriKeyValueRow {
  return {
    id: row.id,
    enabled: row.enabled,
    key: row.key,
    value: row.value,
  };
}

function toFrontendBody(body: TauriRequestBody): ApiClientRequestBody {
  return {
    kind: body.kind,
    text: body.text,
    form: body.form.map(toFrontendRow),
  };
}

function toTauriBody(body: ApiClientRequestBody): TauriRequestBody {
  return {
    kind: body.kind,
    text: body.text,
    form: body.form.map(toTauriRow),
  };
}

function toFrontendProject(project: TauriProject): ApiClientProject {
  return {
    id: project.id,
    name: project.name,
    description: project.description,
    sort: project.sort,
    createdAt: project.created_at,
    updatedAt: project.updated_at,
  };
}

function toFrontendGroup(group: TauriGroup): ApiClientGroup {
  return {
    id: group.id,
    projectId: group.project_id,
    parentGroupId: group.parent_group_id,
    name: group.name,
    sort: group.sort,
    createdAt: group.created_at,
    updatedAt: group.updated_at,
  };
}

function toFrontendRequest(request: TauriRequest): ApiClientRequest {
  return {
    id: request.id,
    groupId: request.group_id,
    projectId: request.project_id,
    name: request.name,
    method: request.method,
    url: request.url,
    query: request.query.map(toFrontendRow),
    headers: request.headers.map(toFrontendRow),
    body: toFrontendBody(request.body),
    timeoutMs: request.timeout_ms,
    aiPrompt: request.ai_prompt,
    aiReference: request.ai_reference,
    sort: request.sort,
    createdAt: request.created_at,
    updatedAt: request.updated_at,
  };
}

function toTauriRequest(request: ApiClientRequest): TauriRequest {
  return {
    id: request.id,
    group_id: request.groupId,
    project_id: request.projectId,
    name: request.name,
    method: request.method,
    url: request.url,
    query: request.query.map(toTauriRow),
    headers: request.headers.map(toTauriRow),
    body: toTauriBody(request.body),
    timeout_ms: request.timeoutMs,
    ai_prompt: request.aiPrompt,
    ai_reference: request.aiReference,
    sort: request.sort,
    created_at: request.createdAt,
    updated_at: request.updatedAt,
  };
}

function toFrontendEnvironment(env: TauriEnvironment): ApiClientEnvironment {
  return {
    id: env.id,
    projectId: env.project_id,
    name: env.name,
    baseUrl: env.base_url,
    variables: env.variables.map(toFrontendRow),
    isSensitive: env.is_sensitive,
    createdAt: env.created_at,
    updatedAt: env.updated_at,
  };
}

function toTauriEnvironment(env: ApiClientEnvironment): TauriEnvironment {
  return {
    id: env.id,
    project_id: env.projectId,
    name: env.name,
    base_url: env.baseUrl,
    variables: env.variables.map(toTauriRow),
    is_sensitive: env.isSensitive,
    created_at: env.createdAt,
    updated_at: env.updatedAt,
  };
}

function toFrontendSnapshot(snapshot: TauriRequestSnapshot): ApiClientRequestSnapshot {
  return {
    method: snapshot.method,
    url: snapshot.url,
    query: snapshot.query.map(toFrontendRow),
    headers: snapshot.headers.map(toFrontendRow),
    body: toFrontendBody(snapshot.body),
    timeoutMs: snapshot.timeout_ms,
    environmentName: snapshot.environment_name,
  };
}

function toFrontendHistory(history: TauriRequestHistory): ApiClientRequestHistory {
  return {
    id: history.id,
    requestId: history.request_id,
    projectId: history.project_id,
    environmentName: history.environment_name,
    requestSnapshot: toFrontendSnapshot(history.request_snapshot),
    status: history.status as ApiClientRequestHistory['status'],
    statusCode: history.status_code,
    responseHeaders: history.response_headers.map(toFrontendRow),
    responseBodyPreview: history.response_body_preview,
    responseBodyTruncated: history.response_body_truncated,
    durationMs: history.duration_ms,
    errorMessage: history.error_message,
    executedAt: history.executed_at,
  };
}

export async function listApiProjects(): Promise<ApiClientProject[]> {
  const raw = await invoke<TauriProject[]>('api_client_list_projects');
  return raw.map(toFrontendProject);
}

export async function createApiProject(name: string, description: string): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('api_client_create_project', { name, description });
  return toFrontendProject(raw);
}

export async function renameApiProject(projectId: string, name: string): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('api_client_rename_project', { projectId, name });
  return toFrontendProject(raw);
}

export async function updateApiProject(projectId: string, name: string, description: string): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('api_client_update_project', { projectId, name, description });
  return toFrontendProject(raw);
}

export async function deleteApiProject(projectId: string): Promise<{ deletedRequests: number; deletedHistories: number }> {
  const raw = await invoke<{ deleted_requests: number; deleted_histories: number }>(
    'api_client_delete_project',
    { projectId },
  );
  return {
    deletedRequests: raw.deleted_requests,
    deletedHistories: raw.deleted_histories,
  };
}

export async function listApiGroups(projectId: string): Promise<ApiClientGroup[]> {
  const raw = await invoke<TauriGroup[]>('api_client_list_groups', { projectId });
  return raw.map(toFrontendGroup);
}

export async function createApiGroup(projectId: string, name: string): Promise<ApiClientGroup> {
  const raw = await invoke<TauriGroup>('api_client_create_group', { projectId, name });
  return toFrontendGroup(raw);
}

export async function renameApiGroup(groupId: string, name: string): Promise<ApiClientGroup> {
  const raw = await invoke<TauriGroup>('api_client_rename_group', { groupId, name });
  return toFrontendGroup(raw);
}

export async function deleteApiGroup(groupId: string): Promise<{ deletedRequests: number; deletedHistories: number }> {
  const raw = await invoke<{ deleted_requests: number; deleted_histories: number }>(
    'api_client_delete_group',
    { groupId },
  );
  return {
    deletedRequests: raw.deleted_requests,
    deletedHistories: raw.deleted_histories,
  };
}

export async function moveApiGroup(groupId: string, sort: number): Promise<ApiClientGroup> {
  const raw = await invoke<TauriGroup>('api_client_move_group', { groupId, sort });
  return toFrontendGroup(raw);
}

export async function listApiRequests(groupId: string): Promise<ApiClientRequest[]> {
  const raw = await invoke<TauriRequest[]>('api_client_list_requests', { groupId });
  return raw.map(toFrontendRequest);
}

export async function listAllApiRequests(projectId: string): Promise<ApiClientRequest[]> {
  const raw = await invoke<TauriRequest[]>('api_client_list_project_requests', { projectId });
  return raw.map(toFrontendRequest);
}

export async function getApiRequest(requestId: string): Promise<ApiClientRequest | null> {
  const raw = await invoke<TauriRequest | null>('api_client_get_request', { requestId });
  return raw ? toFrontendRequest(raw) : null;
}

export async function createApiRequest(input: {
  groupId: string;
  name: string;
  method: ApiClientHttpMethod;
  url: string;
}): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('api_client_create_request', {
    groupId: input.groupId,
    name: input.name,
    method: input.method,
    url: input.url,
  });
  return toFrontendRequest(raw);
}

export async function saveApiRequest(request: ApiClientRequest): Promise<ApiClientRequest> {
  const tauri = toTauriRequest(request);
  const raw = await invoke<TauriRequest>('api_client_save_request', { request: tauri });
  return toFrontendRequest(raw);
}

export async function duplicateApiRequest(requestId: string): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('api_client_duplicate_request', { requestId });
  return toFrontendRequest(raw);
}

export async function moveApiRequest(requestId: string, groupId: string, sort: number): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('api_client_move_request', { requestId, groupId, sort });
  return toFrontendRequest(raw);
}

export async function deleteApiRequest(requestId: string): Promise<{ deletedHistories: number }> {
  const raw = await invoke<{ deleted_histories: number }>('api_client_delete_request', { requestId });
  return { deletedHistories: raw.deleted_histories };
}

export async function listApiEnvironments(projectId: string): Promise<ApiClientEnvironment[]> {
  const raw = await invoke<TauriEnvironment[]>('api_client_list_environments', { projectId });
  return raw.map(toFrontendEnvironment);
}

export async function createApiEnvironment(input: {
  projectId: string;
  name: string;
  baseUrl: string;
  variables: ApiClientKeyValueRow[];
  isSensitive: boolean;
}): Promise<ApiClientEnvironment> {
  const raw = await invoke<TauriEnvironment>('api_client_create_environment', {
    projectId: input.projectId,
    name: input.name,
    baseUrl: input.baseUrl,
    variables: input.variables.map(toTauriRow),
    isSensitive: input.isSensitive,
  });
  return toFrontendEnvironment(raw);
}

export async function saveApiEnvironment(env: ApiClientEnvironment): Promise<ApiClientEnvironment> {
  const raw = await invoke<TauriEnvironment>('api_client_save_environment', {
    environment: toTauriEnvironment(env),
  });
  return toFrontendEnvironment(raw);
}

export async function deleteApiEnvironment(environmentId: string): Promise<void> {
  await invoke('api_client_delete_environment', { environmentId });
}

export interface TauriExecuteRequestInput {
  requestId: string;
  executionId: string;
  snapshot: ApiClientRequestSnapshot;
  environmentId: string | null;
}

export interface TauriExecuteRequestResult {
  execution_id: string;
  status: string;
  status_code: number | null;
  duration_ms: number;
  size_bytes: number;
  content_type: string | null;
  response_headers: TauriKeyValueRow[];
  response_body: string;
  response_truncated: boolean;
  is_binary: boolean;
  oversize: boolean;
  connection_failed: boolean;
  timed_out: boolean;
  cancelled: boolean;
  error_message: string | null;
  history_id: string | null;
}

export interface ApiExecuteResult {
  executionId: string;
  status: string;
  statusCode: number | null;
  durationMs: number;
  sizeBytes: number;
  contentType: string | null;
  responseHeaders: ApiClientKeyValueRow[];
  responseBody: string;
  responseTruncated: boolean;
  isBinary: boolean;
  oversize: boolean;
  connectionFailed: boolean;
  timedOut: boolean;
  cancelled: boolean;
  errorMessage: string | null;
  historyId: string | null;
}

export async function executeApiRequest(input: TauriExecuteRequestInput): Promise<ApiExecuteResult> {
  const snapshot: TauriRequestSnapshot = {
    method: input.snapshot.method,
    url: input.snapshot.url,
    query: input.snapshot.query.map(toTauriRow),
    headers: input.snapshot.headers.map(toTauriRow),
    body: toTauriBody(input.snapshot.body),
    timeout_ms: input.snapshot.timeoutMs,
    environment_name: input.snapshot.environmentName,
  };

  const raw = await invoke<TauriExecuteRequestResult>('api_client_execute_request', {
    requestId: input.requestId,
    executionId: input.executionId,
    snapshot,
    environmentId: input.environmentId,
  });

  return {
    executionId: raw.execution_id,
    status: raw.status,
    statusCode: raw.status_code,
    durationMs: raw.duration_ms,
    sizeBytes: raw.size_bytes,
    contentType: raw.content_type,
    responseHeaders: raw.response_headers.map(toFrontendRow),
    responseBody: raw.response_body,
    responseTruncated: raw.response_truncated,
    isBinary: raw.is_binary,
    oversize: raw.oversize,
    connectionFailed: raw.connection_failed,
    timedOut: raw.timed_out,
    cancelled: raw.cancelled,
    errorMessage: raw.error_message,
    historyId: raw.history_id,
  };
}

export async function cancelApiExecution(executionId: string): Promise<void> {
  await invoke('api_client_cancel_execution', { executionId });
}

export async function listApiRequestHistory(requestId: string): Promise<ApiClientRequestHistory[]> {
  const raw = await invoke<TauriRequestHistory[]>('api_client_list_history', { requestId });
  return raw.map(toFrontendHistory);
}

export async function clearApiRequestHistory(requestId: string): Promise<void> {
  await invoke('api_client_clear_history', { requestId });
}

export interface TauriAiGenerateInput {
  requestId: string;
  taskId: string;
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  currentBody: ApiClientRequestBody | null;
  requestName: string;
  method: ApiClientHttpMethod;
  url: string;
}

export interface TauriAiGenerateResult {
  task_id: string;
  request_id: string;
  content: string;
  model_label: string;
}

export async function generateApiBody(input: TauriAiGenerateInput): Promise<TauriAiGenerateResult> {
  return invoke<TauriAiGenerateResult>('api_client_generate_body', {
    requestId: input.requestId,
    taskId: input.taskId,
    prompt: input.prompt,
    reference: input.reference,
    includeCurrentBody: input.includeCurrentBody,
    currentBody: input.currentBody ? toTauriBody(input.currentBody) : null,
    requestName: input.requestName,
    method: input.method,
    url: input.url,
  });
}

export async function cancelApiGeneration(taskId: string): Promise<void> {
  await invoke('api_client_cancel_generation', { taskId });
}

export interface TauriAiModelOption {
  provider_id: string;
  model_id: string;
  label: string;
}

export interface ApiAiModelOption {
  providerId: string;
  modelId: string;
  label: string;
}

export async function listApiAiModels(): Promise<ApiAiModelOption[]> {
  const raw = await invoke<TauriAiModelOption[]>('api_client_list_ai_models');
  return raw.map(option => ({
    providerId: option.provider_id,
    modelId: option.model_id,
    label: option.label,
  }));
}

export async function getApiRequestBodyGenerationModel(): Promise<ApiAiModelOption | null> {
  const raw = await invoke<TauriAiModelOption | null>('api_client_get_body_generation_model');
  return raw
    ? {
        providerId: raw.provider_id,
        modelId: raw.model_id,
        label: raw.label,
      }
    : null;
}

export async function isApiBackendAvailable(): Promise<boolean> {
  try {
    await invoke('api_client_list_projects');
    return true;
  } catch {
    return false;
  }
}

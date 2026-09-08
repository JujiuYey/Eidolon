import { invoke } from '@tauri-apps/api/core';
import type {
  ApiClientBodyKind,
  ApiClientEnvironment,
  ApiClientGroup,
  ApiClientHttpMethod,
  ApiClientKeyValueRow,
  ApiClientProject,
  ApiClientRequest,
  ApiClientRequestHistory,
  ApiClientRequestSnapshot,
  ApiClientResponseKind,
} from '@/types/api-client';

interface TauriKeyValueRow {
  id: string;
  enabled: boolean;
  key: string;
  value: string;
}

type TauriBodyKind = 'none' | 'json' | 'text' | 'form';

interface TauriRequestBody {
  kind: TauriBodyKind;
  text: string;
  form: TauriKeyValueRow[];
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
  method: TauriHttpMethod;
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

type TauriHttpMethod
  = | 'GET'
    | 'POST'
    | 'PUT'
    | 'PATCH'
    | 'DELETE'
    | 'HEAD'
    | 'OPTIONS';

interface TauriEnvironment {
  id: string;
  project_id: string;
  name: string;
  base_url: string;
  variables: TauriKeyValueRow[];
  created_at: number;
  updated_at: number;
}

type TauriExecutionStatus
  = | 'success'
    | 'http_error'
    | 'network_error'
    | 'timeout'
    | 'cancelled'
    | 'oversize';

interface TauriRequestSnapshot {
  method: string;
  url: string;
  query: TauriKeyValueRow[];
  headers: TauriKeyValueRow[];
  body: TauriRequestBody;
  timeout_ms: number;
  environment_name: string | null;
}

interface TauriRequestHistory {
  id: string;
  request_id: string;
  environment_name: string | null;
  request_snapshot: TauriRequestSnapshot;
  status: TauriExecutionStatus;
  status_code: number | null;
  response_headers: TauriKeyValueRow[];
  response_body_preview: string;
  response_body_truncated: boolean;
  duration_ms: number;
  error_message: string | null;
  executed_at: number;
}

interface TauriExecutionResult {
  execution_id: string;
  status: TauriExecutionStatus;
  status_code: number | null;
  response_headers: TauriKeyValueRow[];
  body_text: string;
  body_size_bytes: number;
  content_type: string | null;
  is_binary: boolean;
  is_oversized: boolean;
  duration_ms: number;
  error_message: string | null;
  history_error: string | null;
  history_id: string | null;
}

interface TauriDeletionSummary {
  deleted_requests: number;
  deleted_histories: number;
}

interface TauriAiGenerateInput {
  prompt: string;
  reference: string;
  current_body: string | null;
  request_name: string;
  method: string;
  url: string;
}

interface TauriAiGenerateResult {
  content: string;
  is_json_valid: boolean;
  json_error: string | null;
  model_label: string;
}

const SUPPORTED_HTTP_METHODS: readonly TauriHttpMethod[] = [
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
];

function isSupportedHttpMethod(value: string): value is TauriHttpMethod {
  return (SUPPORTED_HTTP_METHODS as readonly string[]).includes(value);
}

function toHttpMethod(value: string): ApiClientHttpMethod {
  return isSupportedHttpMethod(value) ? value : 'GET';
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

function toFrontendBody(body: TauriRequestBody): {
  kind: ApiClientBodyKind;
  text: string;
  form: ApiClientKeyValueRow[];
} {
  return {
    kind: body.kind,
    text: body.text,
    form: body.form.map(toFrontendRow),
  };
}

function toTauriBody(body: {
  kind: ApiClientBodyKind;
  text: string;
  form: ApiClientKeyValueRow[];
}): TauriRequestBody {
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
    method: toHttpMethod(request.method),
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
    method: toHttpMethod(request.method),
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
    created_at: env.createdAt,
    updated_at: env.updatedAt,
  };
}

function toFrontendSnapshot(snapshot: TauriRequestSnapshot): ApiClientRequestSnapshot {
  return {
    method: toHttpMethod(snapshot.method),
    url: snapshot.url,
    query: snapshot.query.map(toFrontendRow),
    headers: snapshot.headers.map(toFrontendRow),
    body: toFrontendBody(snapshot.body),
    timeoutMs: snapshot.timeout_ms,
    environmentName: snapshot.environment_name,
  };
}

function toTauriSnapshot(snapshot: ApiClientRequestSnapshot): TauriRequestSnapshot {
  return {
    method: snapshot.method,
    url: snapshot.url,
    query: snapshot.query.map(toTauriRow),
    headers: snapshot.headers.map(toTauriRow),
    body: toTauriBody(snapshot.body),
    timeout_ms: snapshot.timeoutMs,
    environment_name: snapshot.environmentName,
  };
}

function toFrontendHistory(history: TauriRequestHistory): ApiClientRequestHistory {
  return {
    id: history.id,
    requestId: history.request_id,
    environmentName: history.environment_name,
    requestSnapshot: toFrontendSnapshot(history.request_snapshot),
    status: history.status,
    statusCode: history.status_code,
    responseHeaders: history.response_headers.map(toFrontendRow),
    responseBodyPreview: history.response_body_preview,
    responseBodyTruncated: history.response_body_truncated,
    durationMs: history.duration_ms,
    errorMessage: history.error_message,
    executedAt: history.executed_at,
  };
}

export interface ApiExecuteResult {
  executionId: string;
  status: TauriExecutionStatus;
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
  historyError: string | null;
}

function toFrontendExecution(result: TauriExecutionResult): ApiExecuteResult {
  return {
    executionId: result.execution_id,
    status: result.status,
    statusCode: result.status_code,
    durationMs: result.duration_ms,
    sizeBytes: result.body_size_bytes,
    contentType: result.content_type,
    responseHeaders: result.response_headers.map(toFrontendRow),
    responseBody: result.body_text,
    responseTruncated: result.is_oversized,
    isBinary: result.is_binary,
    oversize: result.is_oversized,
    connectionFailed: result.status === 'network_error',
    timedOut: result.status === 'timeout',
    cancelled: result.status === 'cancelled',
    errorMessage: result.error_message,
    historyId: result.history_id,
    historyError: result.history_error,
  };
}

export interface ApiSendRequestInput {
  requestId: string;
  executionId: string;
  snapshot: ApiClientRequestSnapshot;
  environmentId: string | null;
}

export interface ApiAiGenerateInput {
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  currentBody: string | null;
  requestName: string;
  method: string;
  url: string;
}

export interface ApiAiGenerateResult {
  content: string;
  isJsonValid: boolean;
  jsonError: string | null;
  modelLabel: string;
}

function toTauriAiInput(input: ApiAiGenerateInput): TauriAiGenerateInput {
  return {
    prompt: input.prompt,
    reference: input.reference,
    current_body: input.includeCurrentBody ? input.currentBody : null,
    request_name: input.requestName,
    method: input.method,
    url: input.url,
  };
}

function toFrontendAiResult(result: TauriAiGenerateResult): ApiAiGenerateResult {
  return {
    content: result.content,
    isJsonValid: result.is_json_valid,
    jsonError: result.json_error,
    modelLabel: result.model_label,
  };
}

export async function listApiProjects(): Promise<ApiClientProject[]> {
  const raw = await invoke<TauriProject[]>('list_api_projects');
  return raw.map(toFrontendProject);
}

export async function createApiProject(name: string, description: string): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('create_api_project', {
    name,
    description,
  });
  return toFrontendProject(raw);
}

export async function renameApiProject(projectId: string, name: string): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('rename_api_project', { projectId, name });
  return toFrontendProject(raw);
}

export async function updateApiProject(
  projectId: string,
  name: string,
  description: string,
): Promise<ApiClientProject> {
  const raw = await invoke<TauriProject>('update_api_project', { projectId, name, description });
  return toFrontendProject(raw);
}

export async function previewApiProjectDeletion(projectId: string): Promise<{ deletedRequests: number; deletedHistories: number }> {
  const raw = await invoke<TauriDeletionSummary>('preview_api_project_deletion', { projectId });
  return {
    deletedRequests: raw.deleted_requests,
    deletedHistories: raw.deleted_histories,
  };
}

export async function deleteApiProject(projectId: string): Promise<{ deletedRequests: number; deletedHistories: number }> {
  const raw = await invoke<TauriDeletionSummary>('delete_api_project', { projectId });
  return {
    deletedRequests: raw.deleted_requests,
    deletedHistories: raw.deleted_histories,
  };
}

export async function listApiGroups(projectId: string): Promise<ApiClientGroup[]> {
  const raw = await invoke<TauriGroup[]>('list_api_groups', { projectId });
  return raw.map(toFrontendGroup);
}

export async function createApiGroup(projectId: string, name: string): Promise<ApiClientGroup> {
  const raw = await invoke<TauriGroup>('create_api_group', { projectId, name });
  return toFrontendGroup(raw);
}

export async function renameApiGroup(groupId: string, name: string): Promise<ApiClientGroup> {
  const raw = await invoke<TauriGroup>('rename_api_group', { groupId, name });
  return toFrontendGroup(raw);
}

export async function deleteApiGroup(groupId: string): Promise<{ deletedRequests: number; deletedHistories: number }> {
  const raw = await invoke<TauriDeletionSummary>('delete_api_group', { groupId });
  return {
    deletedRequests: raw.deleted_requests,
    deletedHistories: raw.deleted_histories,
  };
}

export async function reorderApiGroups(projectId: string, groupIds: string[]): Promise<ApiClientGroup[]> {
  const raw = await invoke<TauriGroup[]>('reorder_api_groups', { projectId, groupIds });
  return raw.map(toFrontendGroup);
}

export async function listApiRequests(projectId: string): Promise<ApiClientRequest[]> {
  const raw = await invoke<TauriRequest[]>('list_api_requests', { projectId });
  return raw.map(toFrontendRequest);
}

export async function getApiRequest(requestId: string): Promise<ApiClientRequest | null> {
  const raw = await invoke<TauriRequest | null>('get_api_request', { requestId });
  return raw ? toFrontendRequest(raw) : null;
}

export async function createApiRequest(input: {
  groupId: string;
  name: string;
}): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('create_api_request', {
    groupId: input.groupId,
    name: input.name,
  });
  return toFrontendRequest(raw);
}

export async function updateApiRequest(request: ApiClientRequest): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('update_api_request', { request: toTauriRequest(request) });
  return toFrontendRequest(raw);
}

export async function duplicateApiRequest(requestId: string): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('duplicate_api_request', { requestId });
  return toFrontendRequest(raw);
}

export async function moveApiRequest(requestId: string, targetGroupId: string): Promise<ApiClientRequest> {
  const raw = await invoke<TauriRequest>('move_api_request', {
    requestId,
    targetGroupId,
  });
  return toFrontendRequest(raw);
}

export async function deleteApiRequest(requestId: string): Promise<{ deletedHistories: number }> {
  const raw = await invoke<TauriDeletionSummary>('delete_api_request', { requestId });
  return { deletedHistories: raw.deleted_histories };
}

export async function reorderApiRequests(groupId: string, requestIds: string[]): Promise<ApiClientRequest[]> {
  const raw = await invoke<TauriRequest[]>('reorder_api_requests', { groupId, requestIds });
  return raw.map(toFrontendRequest);
}

export async function listApiEnvironments(projectId: string): Promise<ApiClientEnvironment[]> {
  const raw = await invoke<TauriEnvironment[]>('list_api_environments', { projectId });
  return raw.map(toFrontendEnvironment);
}

export async function upsertApiEnvironment(env: ApiClientEnvironment): Promise<ApiClientEnvironment> {
  const raw = await invoke<TauriEnvironment>('upsert_api_environment', { environment: toTauriEnvironment(env) });
  return toFrontendEnvironment(raw);
}

export async function deleteApiEnvironment(environmentId: string): Promise<void> {
  await invoke('delete_api_environment', { environmentId });
}

export async function listApiRequestHistories(requestId: string): Promise<ApiClientRequestHistory[]> {
  const raw = await invoke<TauriRequestHistory[]>('list_api_request_histories', { requestId });
  return raw.map(toFrontendHistory);
}

export async function getApiRequestHistory(historyId: string): Promise<ApiClientRequestHistory | null> {
  const raw = await invoke<TauriRequestHistory | null>('get_api_request_history', { historyId });
  return raw ? toFrontendHistory(raw) : null;
}

export async function clearApiRequestHistories(requestId: string): Promise<number> {
  const cleared = await invoke<number>('clear_api_request_histories', { requestId });
  return cleared;
}

export async function sendApiRequest(input: ApiSendRequestInput): Promise<ApiExecuteResult> {
  const raw = await invoke<TauriExecutionResult>('send_api_request', {
    requestId: input.requestId,
    environmentId: input.environmentId,
    snapshot: toTauriSnapshot(input.snapshot),
    executionId: input.executionId,
  });
  return toFrontendExecution(raw);
}

export async function cancelApiRequest(executionId: string): Promise<boolean> {
  return invoke<boolean>('cancel_api_request', { executionId });
}

export async function previewApiRequest(input: {
  requestId: string;
  environmentId: string | null;
  snapshot: ApiClientRequestSnapshot;
}): Promise<{
  method: string;
  url: string;
  bodySizeBytes: number;
  environmentName: string | null;
}> {
  const raw = await invoke<{
    method: string;
    url: string;
    body_size_bytes: number;
    environment_name: string | null;
  }>('preview_api_request', {
    requestId: input.requestId,
    environmentId: input.environmentId,
    snapshot: toTauriSnapshot(input.snapshot),
  });
  return {
    method: raw.method,
    url: raw.url,
    bodySizeBytes: raw.body_size_bytes,
    environmentName: raw.environment_name,
  };
}

export async function generateApiRequestBody(input: ApiAiGenerateInput): Promise<ApiAiGenerateResult> {
  const raw = await invoke<TauriAiGenerateResult>('generate_api_request_body', { input: toTauriAiInput(input) });
  return toFrontendAiResult(raw);
}

export interface ApiAiModelOption {
  providerId: string;
  modelId: string;
  label: string;
}

export type EnvironmentSensitivity = 'unknown' | 'sensitive';

export function buildEnvironmentSensitivity(env: ApiClientEnvironment | null): EnvironmentSensitivity {
  if (!env) {
    return 'unknown';
  }
  return 'unknown';
}

export function getExecutionKindFromStatus(status: ApiExecuteResult['status']): ApiClientResponseKind {
  if (status === 'cancelled') {
    return 'cancelled';
  }
  if (status === 'timeout') {
    return 'timeout';
  }
  if (status === 'network_error') {
    return 'network_error';
  }
  if (status === 'oversize') {
    return 'oversize';
  }
  if (status === 'success') {
    return 'success';
  }
  return 'http_error';
}

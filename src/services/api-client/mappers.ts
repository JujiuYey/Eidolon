import type {
  ApiAiGenerateInput,
  ApiAiGenerateResult,
  ApiClientBodyKind,
  ApiClientEnvironment,
  ApiClientGroup,
  ApiClientKeyValueRow,
  ApiClientProject,
  ApiClientRequest,
  ApiClientRequestSnapshot,
  ApiClientRequestHistory,
  ApiExecuteResult,
} from '@/types/api-client';
import type {
  TauriAiGenerateInput,
  TauriAiGenerateResult,
  TauriEnvironment,
  TauriExecutionResult,
  TauriGroup,
  TauriKeyValueRow,
  TauriProject,
  TauriRequest,
  TauriRequestBody,
  TauriRequestHistory,
  TauriRequestSnapshot,
} from './types';
import { toHttpMethod } from './types';

export function toFrontendRow(row: TauriKeyValueRow): ApiClientKeyValueRow {
  return {
    id: row.id,
    enabled: row.enabled,
    key: row.key,
    value: row.value,
  };
}

export function toTauriRow(row: ApiClientKeyValueRow): TauriKeyValueRow {
  return {
    id: row.id,
    enabled: row.enabled,
    key: row.key,
    value: row.value,
  };
}

export function toFrontendBody(body: TauriRequestBody): {
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

export function toTauriBody(body: {
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

export function toFrontendProject(project: TauriProject): ApiClientProject {
  return {
    id: project.id,
    name: project.name,
    description: project.description,
    sort: project.sort,
    createdAt: project.created_at,
    updatedAt: project.updated_at,
  };
}

export function toFrontendGroup(group: TauriGroup): ApiClientGroup {
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

export function toFrontendRequest(request: TauriRequest): ApiClientRequest {
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

export function toTauriRequest(request: ApiClientRequest): TauriRequest {
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

export function toFrontendEnvironment(env: TauriEnvironment): ApiClientEnvironment {
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

export function toTauriEnvironment(env: ApiClientEnvironment): TauriEnvironment {
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

export function toFrontendSnapshot(snapshot: TauriRequestSnapshot): ApiClientRequestSnapshot {
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

export function toTauriSnapshot(snapshot: ApiClientRequestSnapshot): TauriRequestSnapshot {
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

export function toFrontendHistory(history: TauriRequestHistory): ApiClientRequestHistory {
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

export function toFrontendExecution(result: TauriExecutionResult): ApiExecuteResult {
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

export function toTauriAiInput(input: ApiAiGenerateInput): TauriAiGenerateInput {
  return {
    prompt: input.prompt,
    reference: input.reference,
    current_body: input.includeCurrentBody ? input.currentBody : null,
    request_name: input.requestName,
    method: input.method,
    url: input.url,
  };
}

export function toFrontendAiResult(result: TauriAiGenerateResult): ApiAiGenerateResult {
  return {
    content: result.content,
    isJsonValid: result.is_json_valid,
    jsonError: result.json_error,
    modelLabel: result.model_label,
  };
}

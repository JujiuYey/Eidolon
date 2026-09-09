export type ApiClientHttpMethod
  = | 'GET'
    | 'POST'
    | 'PUT'
    | 'PATCH'
    | 'DELETE'
    | 'HEAD'
    | 'OPTIONS';

export const API_CLIENT_HTTP_METHODS: ApiClientHttpMethod[] = [
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
];

export type ApiClientBodyKind = 'none' | 'json' | 'text' | 'form';

export const API_CLIENT_BODY_KINDS: ApiClientBodyKind[] = ['none', 'json', 'text', 'form'];

export interface ApiClientKeyValueRow {
  id: string;
  enabled: boolean;
  key: string;
  value: string;
}

export interface ApiClientProject {
  id: string;
  name: string;
  description: string;
  sort: number;
  createdAt: number;
  updatedAt: number;
}

export interface ApiClientGroup {
  id: string;
  projectId: string;
  parentGroupId: string | null;
  name: string;
  sort: number;
  createdAt: number;
  updatedAt: number;
}

export interface ApiClientRequestBody {
  kind: ApiClientBodyKind;
  text: string;
  form: ApiClientKeyValueRow[];
}

export interface ApiClientRequest {
  id: string;
  groupId: string;
  projectId: string;
  name: string;
  method: ApiClientHttpMethod;
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  timeoutMs: number;
  aiPrompt: string;
  aiReference: string;
  sort: number;
  createdAt: number;
  updatedAt: number;
}

export interface ApiClientEnvironment {
  id: string;
  projectId: string;
  name: string;
  baseUrl: string;
  variables: ApiClientKeyValueRow[];
  createdAt: number;
  updatedAt: number;
}

export type ApiClientHistoryExecutionStatus
  = | 'success'
    | 'http_error'
    | 'network_error'
    | 'timeout'
    | 'cancelled'
    | 'oversize';

export type ApiClientResponseKind
  = | 'idle'
    | 'loading'
    | 'success'
    | 'http_error'
    | 'network_error'
    | 'timeout'
    | 'cancelled'
    | 'oversize'
    | 'binary';

export interface ApiClientRequestSnapshot {
  method: ApiClientHttpMethod;
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  timeoutMs: number;
  environmentName: string | null;
}

export interface ApiClientResponseMeta {
  status: number;
  durationMs: number;
  sizeBytes: number;
  contentType: string | null;
  headers: ApiClientKeyValueRow[];
}

export interface ApiClientResponseView {
  kind: ApiClientResponseKind;
  rawText: string;
  formattedJson: string | null;
  isJsonTruncated: boolean;
  isOversized: boolean;
  isBinary: boolean;
  meta: ApiClientResponseMeta | null;
  errorMessage: string | null;
}

export interface ApiClientRequestHistory {
  id: string;
  requestId: string;
  environmentName: string | null;
  requestSnapshot: ApiClientRequestSnapshot;
  status: ApiClientHistoryExecutionStatus;
  statusCode: number | null;
  responseHeaders: ApiClientKeyValueRow[];
  responseBodyPreview: string;
  responseBodyTruncated: boolean;
  durationMs: number;
  errorMessage: string | null;
  executedAt: number;
}

export interface ApiClientAiCandidate {
  requestId: string;
  taskId: string;
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  content: string;
  isJsonValid: boolean;
  jsonError: string | null;
  status: 'generating' | 'success' | 'error' | 'cancelled';
  errorMessage: string | null;
  modelLabel: string;
}

export interface ApiExecuteResult {
  executionId: string;
  status: ApiClientHistoryExecutionStatus;
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

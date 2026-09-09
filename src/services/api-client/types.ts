import type { ApiClientHttpMethod } from '@/types/api-client';

export interface TauriKeyValueRow {
  id: string;
  enabled: boolean;
  key: string;
  value: string;
}

export type TauriBodyKind = 'none' | 'json' | 'text' | 'form';

export interface TauriRequestBody {
  kind: TauriBodyKind;
  text: string;
  form: TauriKeyValueRow[];
}

export interface TauriProject {
  id: string;
  name: string;
  description: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

export interface TauriGroup {
  id: string;
  project_id: string;
  parent_group_id: string | null;
  name: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

export type TauriHttpMethod
  = | 'GET'
    | 'POST'
    | 'PUT'
    | 'PATCH'
    | 'DELETE'
    | 'HEAD'
    | 'OPTIONS';

export interface TauriRequest {
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

export interface TauriEnvironment {
  id: string;
  project_id: string;
  name: string;
  base_url: string;
  variables: TauriKeyValueRow[];
  created_at: number;
  updated_at: number;
}

export type TauriExecutionStatus
  = | 'success'
    | 'http_error'
    | 'network_error'
    | 'timeout'
    | 'cancelled'
    | 'oversize';

export interface TauriRequestSnapshot {
  method: string;
  url: string;
  query: TauriKeyValueRow[];
  headers: TauriKeyValueRow[];
  body: TauriRequestBody;
  timeout_ms: number;
  environment_name: string | null;
}

export interface TauriRequestHistory {
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

export interface TauriExecutionResult {
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

export interface TauriDeletionSummary {
  deleted_requests: number;
  deleted_histories: number;
}

export interface TauriAiGenerateInput {
  prompt: string;
  reference: string;
  current_body: string | null;
  request_name: string;
  method: string;
  url: string;
}

export interface TauriAiGenerateResult {
  content: string;
  is_json_valid: boolean;
  json_error: string | null;
  model_label: string;
}

export const SUPPORTED_HTTP_METHODS: readonly TauriHttpMethod[] = [
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
];

export function isSupportedHttpMethod(value: string): value is TauriHttpMethod {
  return (SUPPORTED_HTTP_METHODS as readonly string[]).includes(value);
}

export function toHttpMethod(value: string): ApiClientHttpMethod {
  return isSupportedHttpMethod(value) ? value : 'GET';
}

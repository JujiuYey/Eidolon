import { invoke } from '@tauri-apps/api/core';
import type {
  ApiClientRequestSnapshot,
  ApiExecuteResult,
  ApiSendRequestInput,
} from '@/types/api-client';
import { toFrontendExecution, toTauriSnapshot } from './mappers';
import type { TauriExecutionResult } from './types';

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

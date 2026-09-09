import { invoke } from '@tauri-apps/api/core';
import type { ApiClientRequestHistory } from '@/types/api-client';
import { toFrontendHistory } from './mappers';
import type { TauriRequestHistory } from './types';

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

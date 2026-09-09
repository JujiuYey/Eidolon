import { invoke } from '@tauri-apps/api/core';
import type { ApiClientRequest } from '@/types/api-client';
import { toFrontendRequest, toTauriRequest } from './mappers';
import type { TauriDeletionSummary, TauriRequest } from './types';

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

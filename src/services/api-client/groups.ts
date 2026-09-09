import { invoke } from '@tauri-apps/api/core';
import type { ApiClientGroup } from '@/types/api-client';
import { toFrontendGroup } from './mappers';
import type { TauriDeletionSummary, TauriGroup } from './types';

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

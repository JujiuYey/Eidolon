import { invoke } from '@tauri-apps/api/core';
import type { ApiClientEnvironment } from '@/types/api-client';
import { toFrontendEnvironment, toTauriEnvironment } from './mappers';
import type { TauriEnvironment } from './types';

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

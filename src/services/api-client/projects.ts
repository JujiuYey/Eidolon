import { invoke } from '@tauri-apps/api/core';
import type { ApiClientProject } from '@/types/api-client';
import { toFrontendProject } from './mappers';
import type { TauriDeletionSummary, TauriProject } from './types';

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

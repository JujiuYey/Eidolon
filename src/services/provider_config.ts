import type {
  ProviderModel,
  ProviderSetting,
} from '@/types/provider';
import { invoke } from '@tauri-apps/api/core';

export async function listProviderSettings(): Promise<ProviderSetting[]> {
  return invoke<ProviderSetting[]>('list_provider_settings');
}

export async function upsertProviderSetting(setting: ProviderSetting): Promise<string> {
  return invoke<string>('upsert_provider_setting', { setting });
}

export async function deleteProviderSetting(providerId: string): Promise<string> {
  return invoke<string>('delete_provider_setting', { providerId });
}

export async function listProviderModels(): Promise<ProviderModel[]> {
  return invoke<ProviderModel[]>('list_provider_models');
}

export async function replaceProviderModels(providerId: string, models: ProviderModel[]): Promise<string> {
  return invoke<string>('replace_provider_models', {
    providerId,
    models,
  });
}

export async function deleteProviderModels(providerId: string): Promise<string> {
  return invoke<string>('delete_provider_models', { providerId });
}

export async function testAiConnection(request: {
  api_key: string;
  base_url: string;
  model: string;
}): Promise<void> {
  await invoke('test_ai_connection', { request });
}

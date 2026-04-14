import { invoke } from '@tauri-apps/api/core';

export interface ProviderConfig {
  provider_id: string;
  enabled: boolean;
  api_key: string;
  base_url: string;
  created_at?: string;
  updated_at?: string;
}

export async function listProviderConfigs(): Promise<ProviderConfig[]> {
  return invoke<ProviderConfig[]>('list_provider_configs');
}

export async function upsertProviderConfig(config: ProviderConfig): Promise<string> {
  return invoke<string>('upsert_provider_config', { config });
}

export async function deleteProviderConfig(providerId: string): Promise<string> {
  return invoke<string>('delete_provider_config', { providerId });
}

export async function fetchProviderModels(params: {
  baseUrl: string;
  apiKey: string;
  apiType: string;
}): Promise<string[]> {
  return invoke<string[]>('fetch_provider_models', {
    baseUrl: params.baseUrl,
    apiKey: params.apiKey,
    apiType: params.apiType,
  });
}

export async function testAiConnection(request: {
  api_key: string;
  base_url: string;
  model: string;
}): Promise<void> {
  await invoke('test_ai_connection', { request });
}

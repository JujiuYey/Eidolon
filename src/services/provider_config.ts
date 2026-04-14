import type { ProviderConfig } from '@/types/provider';
import { invoke } from '@tauri-apps/api/core';

type PersistedProviderConfig = Pick<ProviderConfig, 'provider_id' | 'enabled' | 'api_key' | 'base_url'>;

function toPersistedProviderConfig(config: ProviderConfig): PersistedProviderConfig {
  return {
    provider_id: config.provider_id,
    enabled: config.enabled,
    api_key: config.api_key,
    base_url: config.base_url,
  };
}

export async function listProviderConfigs(): Promise<PersistedProviderConfig[]> {
  return invoke<PersistedProviderConfig[]>('list_provider_configs');
}

export async function upsertProviderConfig(config: ProviderConfig): Promise<string> {
  return invoke<string>('upsert_provider_config', {
    config: toPersistedProviderConfig(config),
  });
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

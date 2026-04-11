import { invoke } from '@tauri-apps/api/core';

export interface ModelConfig {
  id: string;
  name: string;
  api_key: string;
  base_url: string;
  model: string;
  temperature: number;
  top_p: number;
  max_tokens: number | null;
  top_k: number | null;
  is_active: boolean;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export async function listModelConfigs() {
  return invoke<ModelConfig[]>('list_model_configs');
}

export async function createModelConfig(config: ModelConfig) {
  return invoke<string>('create_model_config', { config });
}

export async function updateModelConfig(config: ModelConfig) {
  return invoke('update_model_config', { config });
}

export async function deleteModelConfig(id: string) {
  return invoke('delete_model_config', { id });
}

export async function setDefaultModel(id: string) {
  return invoke('set_default_config', { id });
}

export async function testAiConnection(request: {
  api_key: string;
  base_url: string;
  model: string;
}): Promise<void> {
  await invoke('test_ai_connection', { request });
}

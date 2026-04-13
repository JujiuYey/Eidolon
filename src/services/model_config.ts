import { invoke } from '@tauri-apps/api/core';

/**
 * 提供商类型枚举
 */
export type ProviderType = 'minimax' | 'volcengine' | 'ollama' | 'deepseek';

export interface ModelConfig {
  /** 提供商类型（对应后端 ProviderKey） */
  provider_type: ProviderType;
  /** 提供商是否启用 */
  enabled: boolean;
  /** API 密钥 */
  api_key: string;
  /** API 基础地址 */
  base_url: string;
  /** 当前选中的模型 ID */
  selected_model_id: string;
  /** 模型目录 */
  catalog: {
    items: Array<{
      id: string;
      name?: string;
      enabled: boolean;
      capabilities: {
        chat: boolean;
        vision: boolean;
        tool_call: boolean;
        reasoning: boolean;
        embedding: boolean;
      };
      context_window?: number;
      max_output_tokens?: number;
    }>;
  };
  /** 创建时间 */
  created_at: string;
  /** 更新时间 */
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
  return invoke<string>('delete_model_config', { id });
}

export async function setDefaultModel(id: string) {
  return invoke<string>('set_default_config', { id });
}

export async function testAiConnection(request: {
  api_key: string;
  base_url: string;
  model: string;
}): Promise<void> {
  await invoke('test_ai_connection', { request });
}

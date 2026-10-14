export interface ProviderRegistryItem {
  provider_id: string;
  name: string;
  icon?: string;
  website?: string;
  default_base_url: string;
}

export interface ProviderSetting {
  provider_id: string;
  enabled: boolean;
  api_key: string;
  base_url: string;
}

export interface ProviderModelCapabilities {
  chat: boolean;
  vision: boolean;
  tool_call: boolean;
  reasoning: boolean;
  embedding: boolean;
}

export interface ProviderModel {
  provider_id: string;
  model_id: string;
  capabilities: ProviderModelCapabilities;
}

/** 远端 /models 接口返回的原始条目，尚未写入本地配置 */
export interface FetchedProviderModel {
  modelId: string;
}

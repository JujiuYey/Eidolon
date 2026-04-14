export type ProviderApiType = 'openai-compatible' | 'ollama';

export interface ProviderModel {
  id: string;
  name: string;
  capabilities: {
    chat: boolean;
    vision: boolean;
    tool_call: boolean;
    reasoning: boolean;
    embedding: boolean;
  };
}

export interface ProviderConfig {
  provider_id: string;
  name: string;
  icon?: string;
  website?: string;
  api_type: ProviderApiType;
  default_base_url: string;
  models: ProviderModel[];
  enabled: boolean;
  api_key: string;
  base_url: string;
  is_builtin: boolean;
  is_configured: boolean;
}

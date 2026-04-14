import type { ProviderConfig } from '@/types/provider';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

export const PROVIDER_REGISTRY: ProviderConfig[] = [
  {
    provider_id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    website: 'https://platform.minimaxi.com/',
    api_type: 'openai-compatible',
    default_base_url: 'https://api.minimaxi.com/v1',
    models: [
      {
        id: 'MiniMax-M2.7',
        name: 'MiniMax-M2.7',
        capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false },
      },
      {
        id: 'MiniMax-M2.7-highspeed',
        name: 'MiniMax-M2.7-highspeed',
        capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false },
      },
    ],
    enabled: true,
    api_key: '',
    base_url: 'https://api.minimaxi.com/v1',
    is_builtin: true,
    is_configured: false,
  },
  {
    provider_id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    website: 'https://www.volcengine.com/',
    api_type: 'openai-compatible',
    default_base_url: 'https://ark.cn-beijing.volces.com/api/v3',
    models: [
      {
        id: 'doubao-pro-32k',
        name: 'Doubao Pro 32K',
        capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false },
      },
      {
        id: 'doubao-lite-32k',
        name: 'Doubao Lite 32K',
        capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false },
      },
    ],
    enabled: true,
    api_key: '',
    base_url: 'https://ark.cn-beijing.volces.com/api/v3',
    is_builtin: true,
    is_configured: false,
  },
  {
    provider_id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    website: 'https://www.deepseek.com/',
    api_type: 'openai-compatible',
    default_base_url: 'https://api.deepseek.com/v1',
    models: [
      {
        id: 'deepseek-chat',
        name: 'DeepSeek Chat',
        capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false },
      },
      {
        id: 'deepseek-reasoner',
        name: 'DeepSeek Reasoner',
        capabilities: { chat: true, vision: false, tool_call: false, reasoning: true, embedding: false },
      },
    ],
    enabled: true,
    api_key: '',
    base_url: 'https://api.deepseek.com/v1',
    is_builtin: true,
    is_configured: false,
  },
  {
    provider_id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    website: 'https://ollama.com/',
    api_type: 'ollama',
    default_base_url: 'http://127.0.0.1:11434',
    models: [
      {
        id: 'llama3',
        name: 'Llama 3',
        capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false },
      },
      {
        id: 'qwen2',
        name: 'Qwen 2',
        capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false },
      },
    ],
    enabled: true,
    api_key: '',
    base_url: 'http://127.0.0.1:11434',
    is_builtin: true,
    is_configured: false,
  },
];

export function findProviderConfig(id: string): ProviderConfig | undefined {
  return PROVIDER_REGISTRY.find(provider => provider.provider_id === id);
}

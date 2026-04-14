import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

export interface ProviderModelMeta {
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

export interface ProviderMeta {
  id: string;
  name: string;
  icon?: string;
  website: string;
  defaultBaseUrl: string;
  defaultModels: ProviderModelMeta[];
  apiType: 'openai-compatible' | 'ollama';
}

export const PROVIDER_REGISTRY: ProviderMeta[] = [
  {
    id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    website: 'https://platform.minimaxi.com/',
    defaultBaseUrl: 'https://api.minimaxi.com/v1',
    apiType: 'openai-compatible',
    defaultModels: [
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
      {
        id: 'MiniMax-Text-01',
        name: 'MiniMax-Text-01',
        capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false },
      },
    ],
  },
  {
    id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    website: 'https://www.volcengine.com/',
    defaultBaseUrl: 'https://ark.cn-beijing.volces.com/api/v3',
    apiType: 'openai-compatible',
    defaultModels: [
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
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    website: 'https://www.deepseek.com/',
    defaultBaseUrl: 'https://api.deepseek.com/v1',
    apiType: 'openai-compatible',
    defaultModels: [
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
  },
  {
    id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    website: 'https://ollama.com/',
    defaultBaseUrl: 'http://127.0.0.1:11434',
    apiType: 'ollama',
    defaultModels: [
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
  },
];

export function findProviderMeta(id: string): ProviderMeta | undefined {
  return PROVIDER_REGISTRY.find(provider => provider.id === id);
}

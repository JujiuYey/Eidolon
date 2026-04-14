import type { ProviderRegistryItem } from '@/types/provider';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

export const PROVIDER_REGISTRY: ProviderRegistryItem[] = [
  {
    provider_id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    website: 'https://platform.minimaxi.com/',
    default_base_url: 'https://api.minimaxi.com/v1',
  },
  {
    provider_id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    website: 'https://www.volcengine.com/',
    default_base_url: 'https://ark.cn-beijing.volces.com/api/v3',
  },
  {
    provider_id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    website: 'https://www.deepseek.com/',
    default_base_url: 'https://api.deepseek.com/v1',
  },
  {
    provider_id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    website: 'https://ollama.com/',
    default_base_url: 'http://127.0.0.1:11434',
  },
];

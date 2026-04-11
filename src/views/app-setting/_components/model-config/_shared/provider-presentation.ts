import type { ModelConfig } from '@/services/model_config';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

type ProviderLookupInput = Partial<Pick<ModelConfig, 'name' | 'base_url' | 'model'>> & {
  vendorId?: string | null;
};

export interface ProviderPresentation {
  id: string;
  name: string;
  initials: string;
  icon?: string;
  iconFallbackClass: string;
  keywords: string[];
}

export interface ProviderModelGroup {
  title: string;
  models: string[];
}

const KNOWN_PROVIDERS: ProviderPresentation[] = [
  {
    id: 'minimax',
    name: 'MiniMax',
    initials: 'MM',
    icon: minimaxIcon,
    iconFallbackClass: 'bg-rose-50 text-rose-500',
    keywords: ['minimax', 'minimaxi'],
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    initials: 'DS',
    icon: deepseekIcon,
    iconFallbackClass: 'bg-indigo-50 text-indigo-500',
    keywords: ['deepseek'],
  },
  {
    id: 'volcengine',
    name: '火山引擎',
    initials: '火山',
    icon: volcengineIcon,
    iconFallbackClass: 'bg-orange-50 text-orange-500',
    keywords: ['volcengine', 'ark', 'doubao', '火山'],
  },
  {
    id: 'ollama',
    name: 'Ollama',
    initials: 'OL',
    icon: ollamaIcon,
    iconFallbackClass: 'bg-slate-100 text-slate-700',
    keywords: ['ollama'],
  },
  {
    id: 'openai',
    name: 'OpenAI',
    initials: 'OA',
    iconFallbackClass: 'bg-emerald-50 text-emerald-600',
    keywords: ['openai'],
  },
  {
    id: 'moonshot',
    name: '月之暗面',
    initials: '月之',
    iconFallbackClass: 'bg-sky-50 text-sky-600',
    keywords: ['moonshot', 'kimi', '月之暗面'],
  },
  {
    id: 'dashscope-compatible',
    name: '通义千问',
    initials: '千问',
    iconFallbackClass: 'bg-violet-50 text-violet-600',
    keywords: ['dashscope', 'qwen', '通义千问'],
  },
  {
    id: 'zhipu',
    name: '智谱 GLM',
    initials: 'GL',
    iconFallbackClass: 'bg-cyan-50 text-cyan-600',
    keywords: ['zhipu', 'bigmodel', 'glm', '智谱'],
  },
];

function normalizeText(value = '') {
  return value.toLowerCase().replace(/[\s/_.:-]+/g, '');
}

function buildFallbackPresentation(name: string): ProviderPresentation {
  return {
    id: normalizeText(name) || 'custom',
    name,
    initials: getInitials(name),
    iconFallbackClass: 'bg-neutral-100 text-neutral-600',
    keywords: [],
  };
}

function getInitials(name: string) {
  const trimmed = name.trim();

  if (!trimmed) {
    return 'AI';
  }

  const englishParts = trimmed
    .split(/[\s_-]+/)
    .map(part => part.replace(/[^a-z0-9]/gi, ''))
    .filter(Boolean);

  if (englishParts.length > 0) {
    return englishParts
      .map(part => part[0]!)
      .join('')
      .slice(0, 2)
      .toUpperCase();
  }

  return trimmed.slice(0, 2);
}

export function resolveProviderPresentation(input: ProviderLookupInput): ProviderPresentation {
  const haystack = [
    input.vendorId,
    input.name,
    input.base_url,
    input.model,
  ]
    .filter(Boolean)
    .map(value => normalizeText(String(value)))
    .join(' ');

  const matched = KNOWN_PROVIDERS.find(provider =>
    provider.keywords.some(keyword => haystack.includes(normalizeText(keyword))),
  );

  if (matched) {
    return matched;
  }

  return buildFallbackPresentation(input.name || '模型平台');
}

function resolveModelGroupTitle(model: string) {
  const normalized = model
    .replace(/^(minimax|deepseek|moonshot|qwen|glm|gpt|ollama)[-_]?/i, '')
    .trim();
  const [segment = '常用模型'] = normalized.split('-');

  if (/^(chat|reasoner|flash|max|mini|plus|turbo|latest|highspeed)$/i.test(segment)) {
    return '常用模型';
  }

  if (/\d/.test(segment)) {
    return segment.toUpperCase();
  }

  return segment.length <= 4 ? segment.toUpperCase() : '常用模型';
}

export function buildModelGroups(models: string[]): ProviderModelGroup[] {
  const uniqueModels = [...new Set(models.filter(Boolean))];

  if (uniqueModels.length === 0) {
    return [];
  }

  const grouped = new Map<string, string[]>();

  uniqueModels.forEach(model => {
    const groupTitle = resolveModelGroupTitle(model);
    grouped.set(groupTitle, [...(grouped.get(groupTitle) ?? []), model]);
  });

  return Array.from(grouped.entries()).map(([title, groupModels]) => ({
    title,
    models: groupModels,
  }));
}

import type { ProviderType } from '@/services/model_config';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

/**
 * 提供商图标映射
 */
export const PROVIDER_ICONS: Record<ProviderType, string> = {
  minimax: minimaxIcon,
  volcengine: volcengineIcon,
  deepseek: deepseekIcon,
  ollama: ollamaIcon,
};

/**
 * 提供商名称映射
 */
export const PROVIDER_NAMES: Record<ProviderType, string> = {
  minimax: 'MiniMax',
  volcengine: '火山引擎',
  deepseek: 'DeepSeek',
  ollama: 'Ollama',
};

/**
 * 提供商后备样式映射
 */
export const PROVIDER_FALLBACK_CLASS: Record<ProviderType, string> = {
  minimax: 'bg-rose-50 text-rose-500',
  volcengine: 'bg-orange-50 text-orange-500',
  deepseek: 'bg-indigo-50 text-indigo-500',
  ollama: 'bg-slate-100 text-slate-700',
};

/**
 * 根据 provider_type 获取图标
 */
export function getProviderIcon(providerType: ProviderType): string | undefined {
  return PROVIDER_ICONS[providerType];
}

/**
 * 根据 provider_type 获取提供商名称
 */
export function getProviderName(providerType: ProviderType): string {
  return PROVIDER_NAMES[providerType];
}

/**
 * 根据 provider_type 获取后备样式
 */
export function getProviderFallbackClass(providerType: ProviderType): string {
  return PROVIDER_FALLBACK_CLASS[providerType];
}

import type { Theme, ThemeColor } from '@/types';

export interface ThemeColorOption {
  value: ThemeColor;
  label: string;
  preview: string;
  indicator: string;
}

export const DEFAULT_THEME_COLOR: ThemeColor = 'blue';

export const THEME_COLOR_OPTIONS: ThemeColorOption[] = [
  { value: 'emerald', label: '翡翠绿', preview: '#10b981', indicator: '#ffffff' },
  { value: 'rose', label: '玫瑰红', preview: '#f43f5e', indicator: '#ffffff' },
  { value: 'teal', label: '湖水青', preview: '#14b8a6', indicator: '#ffffff' },
  { value: 'indigo', label: '靛青蓝', preview: '#6366f1', indicator: '#ffffff' },
  { value: 'violet', label: '紫罗兰', preview: '#8b5cf6', indicator: '#ffffff' },
  { value: 'pink', label: '莓果粉', preview: '#ec4899', indicator: '#ffffff' },
  { value: 'blue', label: '湛蓝', preview: '#3b82f6', indicator: '#ffffff' },
  { value: 'amber', label: '琥珀橙', preview: '#f59e0b', indicator: '#111827' },
  { value: 'purple', label: '葡萄紫', preview: '#7c3aed', indicator: '#ffffff' },
  { value: 'sky', label: '天青蓝', preview: '#0ea5e9', indicator: '#ffffff' },
];

const THEME_COLOR_VALUES = new Set<ThemeColor>(
  THEME_COLOR_OPTIONS.map(option => option.value),
);

export function isThemeColor(value: unknown): value is ThemeColor {
  return typeof value === 'string' && THEME_COLOR_VALUES.has(value as ThemeColor);
}

export function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') {
    return;
  }

  if (theme === 'system') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.classList.toggle('dark', prefersDark);
    return;
  }

  document.documentElement.classList.toggle('dark', theme === 'dark');
}

export function applyThemeColor(themeColor: ThemeColor) {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.dataset.themeColor = themeColor;
}

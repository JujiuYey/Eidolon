import type { Theme, ThemeColor } from '@/types';

export interface ThemeColorOption {
  value: ThemeColor;
  label: string;
  preview: string;
}

export const DEFAULT_THEME_COLOR: ThemeColor = 'indigo';

export const THEME_COLOR_OPTIONS: ThemeColorOption[] = [
  { value: 'rose', label: '玫瑰红', preview: '#f43f5e' },
  { value: 'orange', label: '橙红色', preview: '#f97316' },
  { value: 'amber', label: '琥珀橙', preview: '#f59e0b' },
  { value: 'yellow', label: '正黄', preview: '#facc15' },
  { value: 'emerald', label: '翡翠绿', preview: '#10b981' },
  { value: 'teal', label: '青色', preview: '#14b8a6' },
  { value: 'blue', label: '湛蓝', preview: '#3b82f6' },
  { value: 'indigo', label: '靛青蓝', preview: '#6366f1' },
  { value: 'purple', label: '葡萄紫', preview: '#7c3aed' },
  { value: 'pink', label: '莓果粉', preview: '#ec4899' },
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

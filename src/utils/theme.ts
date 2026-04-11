import type { Theme } from '@/types';

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

import { computed } from 'vue';
import { useAppStore } from '@/stores/app';
import type { Theme, ThemeColor } from '@/types';

export function useTheme() {
  const appStore = useAppStore();

  const setTheme = (newTheme: Theme) => {
    appStore.setTheme(newTheme);
  };

  const setThemeColor = (newThemeColor: ThemeColor) => {
    appStore.setThemeColor(newThemeColor);
  };

  return {
    theme: computed(() => appStore.settings.theme),
    themeColor: computed(() => appStore.settings.themeColor),
    setTheme,
    setThemeColor,
  };
}

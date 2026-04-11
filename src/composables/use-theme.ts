import { computed } from 'vue';
import { useAppStore } from '@/stores/app';
import type { Theme } from '@/types';

export function useTheme() {
  const appStore = useAppStore();

  const setTheme = (newTheme: Theme) => {
    appStore.setTheme(newTheme);
  };

  return {
    theme: computed(() => appStore.settings.theme),
    setTheme,
  };
}

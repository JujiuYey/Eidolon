import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppSettings, Theme, ThemeColor } from '@/types';
import { applyTheme, applyThemeColor, DEFAULT_THEME_COLOR, isThemeColor } from '@/utils/theme';

function createDefaultSettings(): AppSettings {
  return {
    theme: 'system',
    themeColor: DEFAULT_THEME_COLOR,
  };
}

function normalizeSettings(settings: Partial<AppSettings>): AppSettings {
  return {
    theme: settings.theme === 'light' || settings.theme === 'dark' || settings.theme === 'system'
      ? settings.theme
      : 'system',
    themeColor: isThemeColor(settings.themeColor) ? settings.themeColor : DEFAULT_THEME_COLOR,
  };
}

function applyAppearance(settings: AppSettings) {
  applyTheme(settings.theme);
  applyThemeColor(settings.themeColor);
}

export const useAppStore = defineStore('app', () => {
  // 状态 (state)
  const settings = ref<AppSettings>(createDefaultSettings());
  const isInitialized = ref(false);

  const updateSettings = (partialSettings: Partial<AppSettings>) => {
    settings.value = normalizeSettings({
      ...settings.value,
      ...partialSettings,
    });

    applyAppearance(settings.value);
  };

  const setTheme = (theme: Theme) => {
    updateSettings({ theme });
  };

  const setThemeColor = (themeColor: ThemeColor) => {
    updateSettings({ themeColor });
  };

  const init = () => {
    if (isInitialized.value) {
      return;
    }

    applyAppearance(settings.value);
    isInitialized.value = true;
  };

  return {
    settings,
    setTheme,
    setThemeColor,
    init,
  };
}, {
  persist: {
    key: 'eidolon-app-settings',
    pick: ['settings'],
  },
});

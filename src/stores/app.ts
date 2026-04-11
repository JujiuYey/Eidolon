import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppSettings, Theme } from '@/types';
import { loadAppSettingsFromRust, saveAppSettingsToRust } from '@/services/app-settings';
import { applyTheme } from '@/utils/theme';

function createDefaultSettings(): AppSettings {
  return {
    autoSave: true,
    theme: 'system',
    projectPath: null,
    storageDir: null,
    projectFilesExtensions: '.ts,.tsx,.js,.jsx,.vue',
    projectFilesIgnoreDirs: 'node_modules,.git,dist',
    projectFilesMaxFileContentLength: 15000,
  };
}

function normalizeSettings(settings: AppSettings): AppSettings {
  return {
    autoSave: settings.autoSave,
    theme: settings.theme,
    projectPath: settings.projectPath ?? null,
    storageDir: settings.storageDir ?? null,
    projectFilesExtensions: settings.projectFilesExtensions?.trim() || '.ts,.tsx,.js,.jsx,.vue',
    projectFilesIgnoreDirs: settings.projectFilesIgnoreDirs?.trim() || 'node_modules,.git,dist',
    projectFilesMaxFileContentLength: Math.max(1, Number(settings.projectFilesMaxFileContentLength) || 15000),
  };
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

    if (partialSettings.theme) {
      applyTheme(partialSettings.theme);
    }
  };

  const setTheme = (theme: Theme) => {
    updateSettings({ theme });
  };

  const resetSettings = () => {
    settings.value = createDefaultSettings();
    applyTheme(settings.value.theme);
  };

  const init = async () => {
    if (isInitialized.value) {
      return;
    }

    try {
      settings.value = normalizeSettings(await loadAppSettingsFromRust());
    } catch (error) {
      console.error('加载应用设置失败:', error);
      settings.value = createDefaultSettings();
    } finally {
      applyTheme(settings.value.theme);
      isInitialized.value = true;
    }
  };

  const saveToRust = async () => {
    settings.value = normalizeSettings(await saveAppSettingsToRust(settings.value));
    applyTheme(settings.value.theme);
  };

  return {
    // 状态
    settings,
    isInitialized,
    updateSettings,
    setTheme,
    resetSettings,
    init,
    saveToRust,
  };
});

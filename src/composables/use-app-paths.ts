import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface AppPaths {
  app_data: string;
  app_log: string;
}

export function useAppPaths() {
  const appDataPath = ref('');
  const appLogPath = ref('');
  const isLoading = ref(true);
  const error = ref<string | null>(null);

  async function fetchPaths() {
    try {
      isLoading.value = true;
      error.value = null;

      const paths = await invoke<AppPaths>('get_app_paths');
      appDataPath.value = paths.app_data;
      appLogPath.value = paths.app_log;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error('Failed to get app paths:', e);
    } finally {
      isLoading.value = false;
    }
  }

  async function openDirectory(path: string) {
    try {
      await invoke('open_directory', { path });
    } catch (e) {
      console.error('Failed to open directory:', e);
    }
  }

  onMounted(() => {
    fetchPaths();
  });

  return {
    appDataPath,
    appLogPath,
    isLoading,
    error,
    openDirectory,
    refreshPaths: fetchPaths,
  };
}

import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '@/types';

export async function loadAppSettingsFromRust(): Promise<AppSettings> {
  return invoke<AppSettings>('load_app_settings');
}

export async function saveAppSettingsToRust(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>('save_app_settings', { settings });
}

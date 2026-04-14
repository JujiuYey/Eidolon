import type { DefaultModelSetting } from '@/types/default-model';
import { invoke } from '@tauri-apps/api/core';

export async function listDefaultModelSettings(): Promise<DefaultModelSetting[]> {
  return invoke<DefaultModelSetting[]>('list_default_model_settings');
}

export async function upsertDefaultModelSetting(setting: DefaultModelSetting): Promise<string> {
  return invoke<string>('upsert_default_model_setting', { setting });
}

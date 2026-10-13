import { getVersion } from '@tauri-apps/api/app';

/** 应用自身信息。版本号单一来源是 src-tauri/tauri.conf.json 的 version 字段 */
export function getAppVersion(): Promise<string> {
  return getVersion();
}

import { invoke } from '@tauri-apps/api/core';

import type { CodeImportPayload, TauriImportSource, TauriImportSummary } from './types';

/** 在后端仓库执行框架导出命令，返回清单 JSON 文本 */
export async function exportCodeManifest(input: {
  framework: string;
  repoPath: string;
  exportId: string;
}): Promise<string> {
  return invoke<string>('export_code_manifest', {
    framework: input.framework,
    repoPath: input.repoPath,
    exportId: input.exportId,
  });
}

/** 取消正在进行的导出任务 */
export async function cancelCodeExport(exportId: string): Promise<boolean> {
  return invoke<boolean>('cancel_code_export', { exportId });
}

/** 查询项目绑定的导入源（每项目至多一条） */
export async function listApiImportSources(projectId: string): Promise<TauriImportSource[]> {
  return invoke<TauriImportSource[]>('list_api_import_sources', { projectId });
}

/** 执行导入：重建分组并批量写入请求，同时保存导入源绑定 */
export async function runApiImport(input: {
  projectId: string;
  framework: string;
  repoPath: string;
  replaceExisting: boolean;
  payload: CodeImportPayload;
}): Promise<TauriImportSummary> {
  return invoke<TauriImportSummary>('run_api_import', {
    projectId: input.projectId,
    framework: input.framework,
    repoPath: input.repoPath,
    replaceExisting: input.replaceExisting,
    groups: input.payload.groups,
    requests: input.payload.requests,
  });
}

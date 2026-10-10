import type { CodeImportPayload, CodeImportPreview } from './types';
import { buildVefImportPayload, parseVefManifest } from './vef-go';

/**
 * 代码导入框架注册表。新增框架（如病案质控、体检系统）时在
 * `CODE_IMPORT_FRAMEWORKS` 追加一项；导出命令模板在 Rust 端
 * `services/code_export.rs` 登记。
 */
export interface CodeImportFramework {
  id: string;
  label: string;
  description: string;
  /** 向导里环境 baseUrl 的预填值 */
  defaultBaseUrl: string;
  /** 把框架导出的清单 JSON 解析为预览树 */
  parseManifest: (manifestJson: string) => CodeImportPreview;
  /** 把预览树 + 勾选翻译为导入清单 */
  buildImportPayload: (preview: CodeImportPreview, selectedResourceKeys: ReadonlySet<string>) => CodeImportPayload;
}

export const CODE_IMPORT_FRAMEWORKS: CodeImportFramework[] = [
  {
    id: 'vef-go',
    label: 'vef-go',
    description: 'Go · vef-framework-go（RPC 接口，自动导出清单）',
    defaultBaseUrl: 'http://127.0.0.1:8100',
    parseManifest: parseVefManifest,
    buildImportPayload: buildVefImportPayload,
  },
];

export function findCodeImportFramework(id: string): CodeImportFramework | null {
  return CODE_IMPORT_FRAMEWORKS.find(framework => framework.id === id) ?? null;
}

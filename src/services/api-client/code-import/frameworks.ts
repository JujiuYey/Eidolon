import type { CodeImportPayload, CodeImportPreview } from './types';
import { buildVefImportPayload, parseVefManifest } from './vef-go';
import { buildJavaImportPayload, parseJavaManifest } from './vef-java';

/**
 * 代码导入框架注册表。新增框架（如体检系统）时在
 * `CODE_IMPORT_FRAMEWORKS` 追加一项；导出策略（跑命令或扫源码）
 * 在 Rust 端 `services/code_export.rs` 分派。
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
  {
    id: 'vef-java',
    label: 'vef-java',
    description: 'Java · Spring Boot + vef 框架（REST 接口，静态解析源码）',
    defaultBaseUrl: 'http://127.0.0.1:8200',
    parseManifest: parseJavaManifest,
    buildImportPayload: buildJavaImportPayload,
  },
];

export function findCodeImportFramework(id: string): CodeImportFramework | null {
  return CODE_IMPORT_FRAMEWORKS.find(framework => framework.id === id) ?? null;
}

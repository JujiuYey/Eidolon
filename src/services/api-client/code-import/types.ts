import type { TauriKeyValueRow, TauriRequestBody } from '../types';

// ============================================================
// vef-go 清单（manifest JSON 的原始形状，仅适配器内部使用）
// ============================================================

export interface VefManifest {
  framework: string;
  resources: VefManifestResource[];
  types: Record<string, VefManifestType>;
}

export interface VefManifestResource {
  name: string;
  kind: string;
  version: string;
  operations: VefManifestOperation[];
}

export interface VefManifestOperation {
  action: string;
  auth?: string;
  permission?: string;
  audit?: boolean;
  timeoutMs?: number;
  rateLimit?: { max: number; periodMs: number };
  params?: string;
}

export interface VefManifestType {
  fields: VefManifestField[];
}

export interface VefManifestField {
  name: string;
  type: string;
  optional?: boolean;
  label?: string;
  validate?: string;
}

// ============================================================
// 框架无关的预览树（向导第二步展示、勾选的数据源）
// ============================================================

export interface CodeImportModule {
  /** 分组键，如 `m:sys` */
  key: string;
  name: string;
  resources: CodeImportResource[];
}

export interface CodeImportResource {
  /** 分组键，如 `r:smp/sys/user` */
  key: string;
  name: string;
  /** 资源全名（信封里的 resource 字段），如 `smp/sys/user` */
  fullName: string;
  operations: CodeImportOperation[];
}

export interface CodeImportOperation {
  /** 请求外部键，如 `smp/sys/user#find_page` */
  externalKey: string;
  action: string;
  auth: string;
  permission: string | null;
  paramsType: string | null;
  /** 参数字段表（来自清单 types），用于预览展示与请求骨架生成 */
  paramsFields: CodeImportParamField[];
}

export interface CodeImportParamField {
  name: string;
  type: string;
  required: boolean;
  /** 清单里的中文名（label tag），可能为空 */
  label: string | null;
}

export interface CodeImportPreview {
  modules: CodeImportModule[];
  totalRequests: number;
}

// ============================================================
// 导入命令的 Tauri DTO（snake_case，与 Rust 端一一对应）
// ============================================================

export interface TauriImportGroupSpec {
  key: string;
  parent_key: string | null;
  name: string;
}

export interface TauriImportRequestSpec {
  group_key: string;
  external_key: string | null;
  name: string;
  method: string;
  url: string;
  query: TauriKeyValueRow[];
  headers: TauriKeyValueRow[];
  body: TauriRequestBody;
  timeout_ms: number;
}

export interface TauriImportSummary {
  created_groups: number;
  created_requests: number;
  deleted_groups: number;
  deleted_requests: number;
  deleted_histories: number;
}

export interface TauriImportSource {
  id: string;
  project_id: string;
  framework: string;
  repo_path: string;
  created_at: number;
  updated_at: number;
}

export interface CodeImportPayload {
  groups: TauriImportGroupSpec[];
  requests: TauriImportRequestSpec[];
}

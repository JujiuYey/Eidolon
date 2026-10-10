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
// vef-java 清单（Rust code_scan_java 输出的原始形状）
// ============================================================

export interface JavaManifest {
  framework: string;
  controllers: JavaControllerDef[];
  forms: Record<string, JavaFormDef>;
  warnings?: string[];
}

export interface JavaControllerDef {
  name: string;
  module: string;
  base_path: string;
  extends_base: boolean;
  update_form: string | null;
  search_form: string | null;
  endpoints: JavaEndpointDef[];
}

export interface JavaEndpointDef {
  name: string;
  method: string;
  path: string;
  source: 'declared' | 'base';
  body_form: string | null;
  query_params: { name: string; required: boolean }[];
}

export interface JavaFormDef {
  fields: { name: string; field_type: string; required: boolean; label: string | null }[];
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
  /** 请求外部键，如 `smp/sys/user#find_page` 或 `/system/department/page#GET` */
  externalKey: string;
  /** 框架语义的操作名：vef-go 为 action，vef-java 为端点方法名 */
  action: string;
  /** 预览树里显示的一行：vef-go 为 action 名，vef-java 为 `GET /page` */
  label: string;
  /** HTTP 方法（大写） */
  method: string;
  /** 相对资源的子路径；RPC 框架（vef-go）没有子路径 */
  subPath: string | null;
  /** 请求 URL（相对路径，执行时按环境 base_url 拼接），如 `/api` 或 `/system/department/page` */
  url: string;
  auth: string;
  permission: string | null;
  paramsType: string | null;
  /** body 字段表（来自清单），用于预览展示与请求骨架生成 */
  paramsFields: CodeImportParamField[];
  /** query 参数骨架（REST 框架）；RPC 框架为空数组 */
  queryFields: CodeImportParamField[];
  /** body 是否为数组包裹（batch 系列 `[ {...} ]`） */
  bodyIsArray: boolean;
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

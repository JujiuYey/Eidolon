/**
 * dhyx-framework 适配器（体检系统 peis 等）：把 Rust `code_scan_java`
 * 扫描出的清单 JSON 翻译成 Eidolon 的分组/请求定义。
 *
 * 与 vef-java 的关键差异：
 * - Sa-Token 鉴权：`Authorization` 头为**裸 token**（无 Bearer 前缀）
 * - 分页 query `page` 从 **1** 起（Rust 扫描已生成 page/size 参数名，这里填值）
 * - 分页/查询端点挂 SearchForm 字段；写操作请求体为 UpdateForm 字段，
 *   `POST /all`（createAll）为数组包裹
 * - 基类端点含路径参数模板（`/{id}`、`/dict/{fieldName}`），调用时替换
 * - swagger @Api/@ApiOperation 中文标签优先作为资源与端点显示名
 * - 登录为表单提交 `POST /auth/oauth/token`（密码同样需 RSA 加密）
 */

import type {
  CodeImportModule,
  CodeImportOperation,
  CodeImportParamField,
  CodeImportPayload,
  CodeImportPreview,
  CodeImportResource,
  JavaControllerDef,
  JavaEndpointDef,
  JavaManifest,
  TauriImportGroupSpec,
  TauriImportRequestSpec,
} from './types';

/** 请求名里的分隔符，与其他适配器一致 */
const NAME_SEPARATOR = ' · ';

/** 该框架分页 page 从 1 起 */
const QUERY_DEFAULTS: Record<string, string> = { page: '1', size: '20' };

/** 请求体为数组包裹的端点 */
const BODY_ARRAY_ENDPOINTS = new Set(['createAll']);

const LOGIN_EXTERNAL_KEY = '/auth/oauth/token#POST';

export function parseDhyxManifest(manifestJson: string): CodeImportPreview {
  let manifest: JavaManifest;
  try {
    manifest = JSON.parse(manifestJson) as JavaManifest;
  } catch (error) {
    throw new Error(`清单不是合法 JSON: ${error instanceof Error ? error.message : error}`);
  }

  if (!Array.isArray(manifest.controllers)) {
    throw new Error('清单缺少 controllers 数组，请确认这是 dhyx 框架项目的仓库根目录');
  }

  const modules = new Map<string, CodeImportResource[]>();
  for (const controller of manifest.controllers) {
    const operations = controller.endpoints
      .map(endpoint => toOperation(controller, endpoint, manifest))
      .sort((a, b) => a.label.localeCompare(b.label));

    if (operations.length === 0) {
      continue;
    }

    const bucket = modules.get(controller.module) ?? [];
    bucket.push({
      key: `r:${controller.base_path}`,
      name: controller.label ?? controller.base_path.split('/').filter(Boolean).pop() ?? controller.name,
      fullName: controller.base_path,
      operations,
    });
    modules.set(controller.module, bucket);
  }

  const sortedModules: CodeImportModule[] = [...modules.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([moduleName, resources]) => ({
      key: `m:${moduleName}`,
      name: moduleName,
      resources: resources.sort((a, b) => a.name.localeCompare(b.name)),
    }));

  const totalRequests = sortedModules.reduce(
    (sum, module) => sum + module.resources.reduce((inner, resource) => inner + resource.operations.length, 0),
    0,
  );

  return { modules: sortedModules, totalRequests };
}

export function buildDhyxImportPayload(
  preview: CodeImportPreview,
  selectedResourceKeys: ReadonlySet<string>,
): CodeImportPayload {
  const groups: TauriImportGroupSpec[] = [];
  const requests: TauriImportRequestSpec[] = [];

  let hasLoginEndpoint = false;

  for (const module of preview.modules) {
    const selected = module.resources.filter(resource => selectedResourceKeys.has(resource.key));
    if (selected.length === 0) {
      continue;
    }

    groups.push({ key: module.key, parent_key: null, name: module.name });
    for (const resource of selected) {
      groups.push({ key: resource.key, parent_key: module.key, name: resource.name });
      for (const operation of resource.operations) {
        if (operation.externalKey === LOGIN_EXTERNAL_KEY) {
          hasLoginEndpoint = true;
        }
        requests.push(toRequestSpec(resource, operation, resource.key));
      }
    }
  }

  // 登录请求补充：清单里已有真实登录端点（如 AuthController 声明的
  // POST /auth/oauth/token）时直接用；否则合成分组，键与清单撞车前先查重
  if (!hasLoginEndpoint) {
    if (!groups.some(group => group.key === 'm:auth')) {
      groups.push({ key: 'm:auth', parent_key: null, name: 'auth' });
    }
    if (!groups.some(group => group.key === 'r:/auth/oauth')) {
      groups.push({ key: 'r:/auth/oauth', parent_key: 'm:auth', name: '认证' });
    }
    requests.push(loginRequestSpec());
  }

  return { groups, requests };
}

function toOperation(
  controller: JavaControllerDef,
  endpoint: JavaEndpointDef,
  manifest: JavaManifest,
): CodeImportOperation {
  const fullPath = joinPath(controller.base_path, endpoint.path);
  const bodyFields = endpoint.body_form ? resolveFormFields(endpoint.body_form, manifest) : [];

  return {
    externalKey: `${fullPath}#${endpoint.method}`,
    action: endpoint.name,
    label: endpoint.label ?? `${endpoint.method} ${normalizeSubPath(endpoint.path)}`,
    method: endpoint.method,
    subPath: endpoint.path,
    url: fullPath,
    auth: 'token',
    permission: null,
    paramsType: endpoint.body_form,
    paramsFields: bodyFields,
    queryFields: resolveQueryFields(controller, endpoint, manifest),
    bodyIsArray: endpoint.body_form !== null && BODY_ARRAY_ENDPOINTS.has(endpoint.name),
  };
}

/** query 骨架：声明端点取 @RequestParam；分页端点（page）追加 SearchForm 字段 */
function resolveQueryFields(
  controller: JavaControllerDef,
  endpoint: JavaEndpointDef,
  manifest: JavaManifest,
): CodeImportParamField[] {
  const fields: CodeImportParamField[] = endpoint.query_params.map(param => ({
    name: param.name,
    type: param.name === 'page' || param.name === 'size' ? 'number' : 'string',
    required: param.required,
    label: null,
  }));

  if (endpoint.source === 'base' && endpoint.name === 'page' && controller.search_form) {
    fields.push(...resolveFormFields(controller.search_form, manifest).map(field => ({ ...field, required: false })));
  }

  return fields;
}

function resolveFormFields(formName: string, manifest: JavaManifest): CodeImportParamField[] {
  const form = manifest.forms?.[formName];
  if (!form || !Array.isArray(form.fields)) {
    return [];
  }
  return form.fields.map(field => ({
    name: field.name,
    type: field.field_type,
    required: field.required,
    label: field.label,
  }));
}

function toRequestSpec(
  resource: CodeImportResource,
  operation: CodeImportOperation,
  groupKey: string,
): TauriImportRequestSpec {
  // Sa-Token：Authorization 为裸 token，无 Bearer 前缀
  const headers = [{ id: '', enabled: true, key: 'Authorization', value: '{{token}}' }];

  const query = operation.queryFields.map(field => ({
    id: '',
    enabled: true,
    key: field.name,
    value: QUERY_DEFAULTS[field.name] ?? '',
  }));

  let bodyKind: 'json' | 'none' = 'none';
  let bodyText = '';
  const form: { id: string; enabled: boolean; key: string; value: string }[] = [];
  if (operation.paramsType) {
    bodyKind = 'json';
    const skeleton: Record<string, unknown> = {};
    for (const field of operation.paramsFields) {
      skeleton[field.name] = placeholderForType(field.type);
    }
    bodyText = operation.bodyIsArray ? JSON.stringify([skeleton], null, 2) : JSON.stringify(skeleton, null, 2);
  }

  const actionLabel =
    operation.label === `${operation.method} ${operation.subPath ?? ''}`.trim() ? operation.action : operation.label;

  return {
    group_key: groupKey,
    external_key: operation.externalKey,
    name: `${resource.name}${NAME_SEPARATOR}${actionLabel}`,
    method: operation.method,
    url: operation.url,
    query,
    headers,
    body: { kind: bodyKind, text: bodyText, form },
    timeout_ms: 30_000,
  };
}

/** 登录：POST /auth/oauth/token，@RequestParam 表单提交；密码需前端 RSA 加密 */
function loginRequestSpec(): TauriImportRequestSpec {
  return {
    group_key: 'r:/auth/oauth',
    external_key: LOGIN_EXTERNAL_KEY,
    name: 'auth · 登录（密码需 RSA 加密）',
    method: 'POST',
    url: '/auth/oauth/token',
    query: [],
    headers: [],
    body: {
      kind: 'form',
      text: '',
      form: [
        { id: 'login-org', enabled: true, key: 'orgId', value: '' },
        { id: 'login-user', enabled: true, key: 'username', value: '' },
        { id: 'login-pass', enabled: true, key: 'password', value: '' },
      ],
    },
    timeout_ms: 30_000,
  };
}

function joinPath(basePath: string, subPath: string): string {
  if (subPath === '/' || subPath === '') {
    return basePath;
  }
  return `${basePath.replace(/\/$/, '')}${normalizeSubPath(subPath)}`;
}

function normalizeSubPath(subPath: string): string {
  if (!subPath) {
    return '/';
  }
  return subPath.startsWith('/') ? subPath : `/${subPath}`;
}

/** Java 声明类型 → 骨架占位值 */
function placeholderForType(type: string): unknown {
  if (type.startsWith('List<') || type.startsWith('Set<') || type.endsWith('[]')) {
    return [];
  }
  switch (type) {
    case 'String':
    case 'Character':
    case 'char':
      return '';
    case 'Integer':
    case 'int':
    case 'Long':
    case 'long':
    case 'Double':
    case 'double':
    case 'Float':
    case 'float':
    case 'BigDecimal':
    case 'BigInteger':
      return 0;
    case 'Boolean':
    case 'boolean':
      return false;
    default:
      // 枚举 / 嵌套对象：null 占位
      return null;
  }
}

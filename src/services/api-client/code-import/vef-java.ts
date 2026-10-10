/**
 * vef-java 适配器：把 Rust `code_scan_java` 扫描出的清单 JSON 翻译成
 * Eidolon 的分组/请求定义。
 *
 * vef-java（Spring Boot）是 REST 风格：URL = base_path + 子路径，
 * 查询条件走 query 参数（分页 `page` 从 0 起 + `size`），写操作
 * 的请求体是 UpdateForm 字段，batch 系列为数组包裹。所有业务端点
 * 需 Bearer token；适配器额外自动生成一条「登录」请求方便取 token
 * （注意：该项目登录密码需前端 RSA 加密，工具内发明文会失败）。
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

/** 请求名里的分隔符，与 vef-go 一致，如 `department · fetchPage` */
const NAME_SEPARATOR = ' · ';

/** 需要注入 SearchForm 查询字段的基类端点 */
const SEARCH_ENDPOINTS = new Set(['fetchList', 'fetchPage', 'fetchOptions', 'export']);

/** query 参数的默认占位值 */
const QUERY_DEFAULTS: Record<string, string> = { page: '0', size: '20' };

/** 登录请求（框架内置端点，源码中不存在，由适配器补充） */
const LOGIN_EXTERNAL_KEY = '/auth/login#POST';

export function parseJavaManifest(manifestJson: string): CodeImportPreview {
  let manifest: JavaManifest;
  try {
    manifest = JSON.parse(manifestJson) as JavaManifest;
  } catch (error) {
    throw new Error(`清单不是合法 JSON: ${error instanceof Error ? error.message : error}`);
  }

  if (!Array.isArray(manifest.controllers)) {
    throw new Error('清单缺少 controllers 数组，请确认这是 vef-java 项目的仓库根目录');
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

export function buildJavaImportPayload(
  preview: CodeImportPreview,
  selectedResourceKeys: ReadonlySet<string>,
): CodeImportPayload {
  const groups: TauriImportGroupSpec[] = [];
  const requests: TauriImportRequestSpec[] = [];

  // 登录请求无条件补充：取 token 是调用其他接口的前提
  groups.push({ key: 'm:auth', parent_key: null, name: 'auth' });
  groups.push({ key: 'r:/auth', parent_key: 'm:auth', name: 'auth' });
  requests.push(loginRequestSpec());

  for (const module of preview.modules) {
    const selected = module.resources.filter(resource => selectedResourceKeys.has(resource.key));
    if (selected.length === 0) {
      continue;
    }

    groups.push({ key: module.key, parent_key: null, name: module.name });
    for (const resource of selected) {
      groups.push({ key: resource.key, parent_key: module.key, name: resource.name });
      for (const operation of resource.operations) {
        requests.push(toRequestSpec(resource, operation, resource.key));
      }
    }
  }

  return { groups, requests };
}

function toOperation(
  controller: JavaControllerDef,
  endpoint: JavaEndpointDef,
  manifest: JavaManifest,
): CodeImportOperation {
  const fullPath = joinPath(controller.base_path, endpoint.path);
  const queryFields = resolveQueryFields(controller, endpoint, manifest);
  const bodyFields = endpoint.body_form ? resolveFormFields(endpoint.body_form, manifest) : [];
  const bodyIsArray = endpoint.body_form !== null && ['batchCreate', 'batchModify'].includes(endpoint.name);

  return {
    externalKey: `${fullPath}#${endpoint.method}`,
    action: endpoint.name,
    label: endpoint.label ?? `${endpoint.method} ${normalizeSubPath(endpoint.path)}`,
    method: endpoint.method,
    subPath: endpoint.path,
    url: fullPath,
    auth: 'bearer',
    permission: null,
    paramsType: endpoint.body_form,
    paramsFields: bodyFields,
    queryFields,
    bodyIsArray,
  };
}

/** query 骨架：声明端点取 @RequestParam；基类查询端点注入 SearchForm 字段 */
function resolveQueryFields(
  controller: JavaControllerDef,
  endpoint: JavaEndpointDef,
  manifest: JavaManifest,
): CodeImportParamField[] {
  const fields: CodeImportParamField[] = endpoint.query_params.map(param => ({
    name: param.name,
    type: 'string',
    required: param.required,
    label: null,
  }));

  if (endpoint.source === 'base' && SEARCH_ENDPOINTS.has(endpoint.name)) {
    if (endpoint.name === 'fetchPage') {
      // 该项目分页 page 从 0 起
      fields.push({ name: 'page', type: 'number', required: false, label: null });
      fields.push({ name: 'size', type: 'number', required: false, label: null });
    }
    if (controller.search_form) {
      fields.push(...resolveFormFields(controller.search_form, manifest).map(field => ({ ...field, required: false })));
    }
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
  const headers = [{ id: '', enabled: true, key: 'Authorization', value: 'Bearer {{token}}' }];

  const query = operation.queryFields.map(field => ({
    id: '',
    enabled: true,
    key: field.name,
    value: QUERY_DEFAULTS[field.name] ?? '',
  }));

  let bodyText = '';
  if (operation.paramsType) {
    const skeleton: Record<string, unknown> = {};
    for (const field of operation.paramsFields) {
      skeleton[field.name] = placeholderForType(field.type);
    }
    bodyText = operation.bodyIsArray ? JSON.stringify([skeleton], null, 2) : JSON.stringify(skeleton, null, 2);
  }

  return {
    group_key: groupKey,
    external_key: operation.externalKey,
    name: `${resource.name}${NAME_SEPARATOR}${operation.action}`,
    method: operation.method,
    url: operation.url,
    query,
    headers,
    body: {
      kind: bodyText ? 'json' : 'none',
      text: bodyText,
      form: [],
    },
    timeout_ms: 30_000,
  };
}

function loginRequestSpec(): TauriImportRequestSpec {
  return {
    group_key: 'r:/auth',
    external_key: LOGIN_EXTERNAL_KEY,
    // 该端点密码需前端 RSA 加密，工具内发明文登录会失败
    name: 'auth · login（密码需 RSA 加密）',
    method: 'POST',
    url: '/auth/login',
    query: [],
    headers: [],
    body: {
      kind: 'json',
      text: JSON.stringify({ loginType: 'accountPassword', account: '', password: '' }, null, 2),
      form: [],
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

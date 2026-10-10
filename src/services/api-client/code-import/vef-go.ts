/**
 * vef-go 适配器：把 `VEF_EXPORT_API` 导出的清单 JSON 翻译成
 * Eidolon 的分组/请求定义。
 *
 * vef 的 RPC 接口全部走 `POST /api`，请求体为信封
 * `{resource, action, params, meta}`；本适配器按资源全名建立
 * 「模块 > 资源」两级分组，每个 operation 生成一条请求。
 * 参数骨架放入字段表里的全部字段（必填与可选，按类型给零值占位），
 * 字段级信息（中文名、必填标记）保留在预览树上供展开查看。
 */

import type {
  CodeImportModule,
  CodeImportOperation,
  CodeImportParamField,
  CodeImportPayload,
  CodeImportPreview,
  CodeImportResource,
  TauriImportGroupSpec,
  TauriImportRequestSpec,
  VefManifest,
  VefManifestField,
  VefManifestOperation,
} from './types';

/** RPC 端点：请求 URL 写相对路径，执行时由环境 base_url 拼接 */
const RPC_ENDPOINT = '/api';

/** 请求名里的分隔符，如 `user · find_page` */
const NAME_SEPARATOR = ' · ';

/** 支持分页 meta 的操作名 */
const PAGINATED_ACTIONS = new Set(['find_page']);

export function parseVefManifest(manifestJson: string): CodeImportPreview {
  const manifest = parseManifest(manifestJson);

  const modules = new Map<string, CodeImportResource[]>();
  for (const resource of manifest.resources) {
    if (!resource.operations || resource.operations.length === 0) {
      continue;
    }

    const { moduleName, resourceModule } = splitResourceName(resource.name);
    const operations: CodeImportOperation[] = [...resource.operations]
      .sort(byAction)
      .map(operation => toOperation(resource.name, operation, manifest));

    const bucket = modules.get(moduleName) ?? [];
    bucket.push({
      key: `r:${resource.name}`,
      name: resourceModule,
      fullName: resource.name,
      operations,
    });
    modules.set(moduleName, bucket);
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

export function buildVefImportPayload(
  preview: CodeImportPreview,
  selectedResourceKeys: ReadonlySet<string>,
): CodeImportPayload {
  const groups: TauriImportGroupSpec[] = [];
  const requests: TauriImportRequestSpec[] = [];

  for (const module of preview.modules) {
    const selected = module.resources.filter(resource => selectedResourceKeys.has(resource.key));
    if (selected.length === 0) {
      continue;
    }

    groups.push({ key: module.key, parent_key: null, name: module.name });
    for (const resource of selected) {
      groups.push({ key: resource.key, parent_key: module.key, name: resource.name });

      for (const operation of resource.operations) {
        requests.push(toRequestSpec(resource.fullName, operation, resource.key));
      }
    }
  }

  return { groups, requests };
}

function parseManifest(manifestJson: string): VefManifest {
  let manifest: VefManifest;
  try {
    manifest = JSON.parse(manifestJson) as VefManifest;
  } catch (error) {
    throw new Error(`清单不是合法 JSON: ${error instanceof Error ? error.message : error}`);
  }

  if (!Array.isArray(manifest.resources)) {
    throw new Error('清单缺少 resources 数组，请确认导出的框架版本');
  }

  return manifest;
}

/**
 * 资源名 → 模块名 + 展示名。业务资源形如 `smp/sys/user`（模块为第二段），
 * 框架内置资源形如 `security/auth`（模块为第一段）。
 */
function splitResourceName(fullName: string): { moduleName: string; resourceModule: string } {
  const segments = fullName.split('/').filter(Boolean);
  if (segments.length >= 3) {
    return {
      moduleName: segments[1] ?? segments[0] ?? fullName,
      resourceModule: segments[segments.length - 1] ?? fullName,
    };
  }
  if (segments.length === 2) {
    return { moduleName: segments[0] ?? fullName, resourceModule: segments[1] ?? fullName };
  }
  return { moduleName: 'common', resourceModule: segments[0] ?? fullName };
}

function byAction(a: VefManifestOperation, b: VefManifestOperation): number {
  return a.action.localeCompare(b.action);
}

function toOperation(
  resourceName: string,
  operation: VefManifestOperation,
  manifest: VefManifest,
): CodeImportOperation {
  const paramsType = operation.params ?? null;
  return {
    externalKey: `${resourceName}#${operation.action}`,
    action: operation.action,
    label: operation.action,
    method: 'POST',
    subPath: null,
    url: RPC_ENDPOINT,
    auth: operation.auth ?? 'bearer',
    permission: operation.permission ?? null,
    paramsType,
    paramsFields: resolveParamFields(paramsType, manifest),
    queryFields: [],
    bodyIsArray: false,
  };
}

/** 从清单 types 表解析参数字段，保留中文名与必填标记 */
function resolveParamFields(paramsType: string | null, manifest: VefManifest): CodeImportParamField[] {
  if (!paramsType) {
    return [];
  }

  const type = manifest.types?.[paramsType];
  if (!type || !Array.isArray(type.fields)) {
    return [];
  }

  return type.fields.map(field => ({
    name: field.name,
    type: field.type,
    required: isRequired(field),
    label: field.label ?? null,
  }));
}

function toRequestSpec(resourceName: string, operation: CodeImportOperation, groupKey: string): TauriImportRequestSpec {
  const headers = [];
  if (operation.auth === 'bearer') {
    headers.push({ id: '', enabled: true, key: 'Authorization', value: 'Bearer {{token}}' });
  }

  const params: Record<string, unknown> = {};
  for (const field of operation.paramsFields) {
    params[field.name] = placeholderForType(field.type);
  }

  const envelope: Record<string, unknown> = {
    resource: resourceName,
    action: operation.action,
    params,
  };
  if (PAGINATED_ACTIONS.has(operation.action)) {
    envelope.meta = { page: 1, size: 20 };
  }

  const displayName = splitResourceName(resourceName).resourceModule;

  return {
    group_key: groupKey,
    external_key: operation.externalKey,
    name: `${displayName}${NAME_SEPARATOR}${operation.action}`,
    method: 'POST',
    url: RPC_ENDPOINT,
    query: [],
    headers,
    body: {
      kind: 'json',
      text: JSON.stringify(envelope, null, 2),
      form: [],
    },
    timeout_ms: 30_000,
  };
}

function isRequired(field: VefManifestField): boolean {
  const rules = field.validate ?? '';
  return rules.split(',').some(rule => rule.trim() === 'required');
}

function placeholderForType(type: string): unknown {
  if (type.startsWith('[]')) {
    return [];
  }
  switch (type) {
    case 'string':
      return '';
    case 'int':
    case 'int8':
    case 'int16':
    case 'int32':
    case 'int64':
    case 'uint':
    case 'uint64':
    case 'float32':
    case 'float64':
      return 0;
    case 'bool':
      return false;
    default:
      // interface / 自定义结构等：null 占位
      return null;
  }
}

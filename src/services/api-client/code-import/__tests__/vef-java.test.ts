import { describe, expect, it } from 'vitest';

import { buildJavaImportPayload, parseJavaManifest } from '../vef-java';
import { JAVA_MANIFEST_FIXTURE } from './java-manifest.fixture';

const MANIFEST_JSON = JSON.stringify(JAVA_MANIFEST_FIXTURE);

describe('parseJavaManifest', () => {
  it('groups controllers into modules and labels endpoints as METHOD path', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);

    expect(preview.modules.map(module => module.name)).toEqual(['api', 'system']);

    const system = preview.modules.find(module => module.name === 'system');
    expect(system?.key).toBe('m:system');
    const department = system?.resources[0];
    expect(department?.key).toBe('r:/system/department');
    expect(department?.name).toBe('department');

    const labels = department?.operations.map(operation => operation.label);
    expect(labels).toContain('GET /page');
    expect(labels).toContain('GET /department-tree');
    expect(preview.totalRequests).toBe(6);
  });

  it('builds stable external keys from full path and method', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const operations = preview.modules.flatMap(m => m.resources).flatMap(r => r.operations);

    const page = operations.find(operation => operation.action === 'fetchPage');
    expect(page?.externalKey).toBe('/system/department/page#GET');
    expect(page?.url).toBe('/system/department/page');
    expect(page?.queryFields.map(field => field.name)).toEqual(['page', 'size', 'status', 'name']);
  });

  it('rejects invalid manifest json', () => {
    expect(() => parseJavaManifest('not json')).toThrow('清单不是合法 JSON');
    expect(() => parseJavaManifest('{"framework":"v"}')).toThrow('controllers 数组');
  });
});

describe('buildJavaImportPayload', () => {
  it('always prepends a login request without auth header', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const payload = buildJavaImportPayload(preview, new Set());

    expect(payload.requests[0]?.external_key).toBe('/auth/login#POST');
    expect(payload.requests[0]?.url).toBe('/auth/login');
    expect(payload.requests[0]?.headers).toEqual([]);
    expect(JSON.parse(payload.requests[0]?.body.text ?? '{}')).toEqual({
      loginType: 'accountPassword',
      account: '',
      password: '',
    });
    expect(payload.groups.map(group => group.key)).toEqual(['m:auth', 'r:/auth']);
  });

  it('emits paginated query with 0-based page and search fields', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const payload = buildJavaImportPayload(preview, new Set(['r:/system/department']));

    const page = payload.requests.find(request => request.external_key === '/system/department/page#GET');
    expect(page?.method).toBe('GET');
    expect(page?.url).toBe('/system/department/page');
    expect(page?.query.map(row => `${row.key}=${row.value}`)).toEqual(['page=0', 'size=20', 'status=', 'name=']);
    expect(page?.body.kind).toBe('none');
    expect(page?.headers[0]?.key).toBe('Authorization');
  });

  it('builds update-form body skeleton and wraps batch endpoints as array', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const payload = buildJavaImportPayload(preview, new Set(['r:/system/department']));

    const create = payload.requests.find(request => request.external_key === '/system/department#POST');
    expect(create?.url).toBe('/system/department');
    expect(create?.method).toBe('POST');
    expect(JSON.parse(create?.body.text ?? '{}')).toEqual({
      name: '',
      type: null,
      remark: '',
    });

    const batchCreate = payload.requests.find(request => request.external_key === '/system/department/batch#POST');
    expect(JSON.parse(batchCreate?.body.text ?? 'null')).toEqual([{ name: '', type: null, remark: '' }]);
  });

  it('keeps declared endpoints and required query params', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const payload = buildJavaImportPayload(preview, new Set(['r:/system/department']));

    const tree = payload.requests.find(request => request.external_key === '/system/department/department-tree#GET');
    expect(tree?.query).toEqual([{ id: '', enabled: true, key: 'tenantId', value: '' }]);

    const one = payload.requests.find(request => request.external_key === '/system/department#GET');
    expect(one?.query[0]).toEqual({ id: '', enabled: true, key: 'id', value: '' });

    // name 使用「资源名 · 端点名」
    expect(tree?.name).toBe('department · fetchDepartmentTree');
  });

  it('maps java list types to array placeholders', () => {
    const preview = parseJavaManifest(MANIFEST_JSON);
    const payload = buildJavaImportPayload(preview, new Set(['r:/external-api']));

    const sync = payload.requests.find(request => request.external_key === '/external-api/business-data#POST');
    expect(JSON.parse(sync?.body.text ?? '{}')).toEqual({ data: [] });
    expect(payload.groups.map(group => group.key)).toEqual(['m:auth', 'r:/auth', 'm:api', 'r:/external-api']);
  });
});

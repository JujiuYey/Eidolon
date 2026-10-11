import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { buildDhyxImportPayload, parseDhyxManifest } from '../dhyx-java';
import type { JavaManifest } from '../types';

const FIXTURE: JavaManifest = {
  framework: 'java-scan',
  controllers: [
    {
      name: 'OrgController',
      module: 'system',
      base_path: '/system/org',
      label: '机构管理控制器',
      framework_kind: 'dhyx',
      update_form: 'OrgUpdateForm',
      search_form: 'OrgSearchForm',
      endpoints: [
        { name: 'get', method: 'GET', path: '/{id}', source: 'base', body_form: null, query_params: [] },
        {
          name: 'page',
          method: 'GET',
          path: '/',
          source: 'base',
          body_form: null,
          query_params: [
            { name: 'page', required: false },
            { name: 'size', required: false },
          ],
        },
        { name: 'create', method: 'POST', path: '/', source: 'base', body_form: 'OrgUpdateForm', query_params: [] },
        {
          name: 'createAll',
          method: 'POST',
          path: '/all',
          source: 'base',
          body_form: 'OrgUpdateForm',
          query_params: [],
        },
        { name: 'remove', method: 'DELETE', path: '/{id}', source: 'base', body_form: null, query_params: [] },
        {
          name: 'getOrgTree',
          label: '获取机构树',
          method: 'GET',
          path: '/tree',
          source: 'declared',
          body_form: null,
          query_params: [{ name: 'parentId', required: false }],
        },
      ],
    },
  ],
  forms: {
    OrgUpdateForm: {
      fields: [
        { name: 'parentId', field_type: 'String', required: true, label: '父编号' },
        { name: 'type', field_type: 'String', required: true, label: null },
      ],
    },
    OrgSearchForm: {
      fields: [{ name: 'name', field_type: 'String', required: false, label: '名字' }],
    },
  },
};

const MANIFEST_JSON = JSON.stringify(FIXTURE);

describe('parseDhyxManifest', () => {
  it('prefers swagger chinese labels for resources and endpoints', () => {
    const preview = parseDhyxManifest(MANIFEST_JSON);

    expect(preview.modules.map(module => module.name)).toEqual(['system']);
    const org = preview.modules[0]?.resources[0];
    expect(org?.name).toBe('机构管理控制器');
    expect(org?.key).toBe('r:/system/org');

    const tree = org?.operations.find(operation => operation.action === 'getOrgTree');
    expect(tree?.label).toBe('获取机构树');
    expect(tree?.url).toBe('/system/org/tree');

    const page = org?.operations.find(operation => operation.action === 'page');
    expect(page?.url).toBe('/system/org');
    expect(page?.queryFields.map(field => field.name)).toEqual(['page', 'size', 'name']);
  });

  it('rejects invalid manifest json', () => {
    expect(() => parseDhyxManifest('not json')).toThrow('清单不是合法 JSON');
    expect(() => parseDhyxManifest('{"framework":"x"}')).toThrow('controllers 数组');
  });

  it('reuses scanned auth controller instead of synthesizing duplicate groups', () => {
    const withAuth: JavaManifest = {
      framework: 'java-scan',
      controllers: [
        {
          name: 'AuthController',
          module: 'auth',
          base_path: '/auth/oauth',
          label: '认证控制器',
          framework_kind: 'dhyx',
          update_form: null,
          search_form: null,
          endpoints: [
            {
              name: 'token',
              method: 'POST',
              path: '/token',
              source: 'declared',
              body_form: null,
              query_params: [
                { name: 'username', required: true },
                { name: 'password', required: true },
              ],
            },
          ],
        },
      ],
      forms: {},
    };
    const preview = parseDhyxManifest(JSON.stringify(withAuth));
    const payload = buildDhyxImportPayload(preview, new Set(['r:/auth/oauth']));

    // 真实登录端点直接使用，不再合成请求；分组键无重复
    expect(payload.requests.filter(request => request.external_key === '/auth/oauth/token#POST')).toHaveLength(1);
    const keys = payload.groups.map(group => group.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(payload.groups.map(group => group.key)).toEqual(['m:auth', 'r:/auth/oauth']);
  });

  it('still synthesizes login when auth module exists but login endpoint is not selected', () => {
    // auth 模块存在但用户没勾选：合成分组不与已选分组撞键
    const preview = parseDhyxManifest(MANIFEST_JSON);
    const payload = buildDhyxImportPayload(preview, new Set(['r:/system/org']));

    const keys = payload.groups.map(group => group.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(payload.requests.find(request => request.external_key === '/auth/oauth/token#POST')).toBeDefined();
  });
});

describe('buildDhyxImportPayload', () => {
  it('prepends form-encoded login request', () => {
    const preview = parseDhyxManifest(MANIFEST_JSON);
    const payload = buildDhyxImportPayload(preview, new Set());

    expect(payload.requests[0]?.external_key).toBe('/auth/oauth/token#POST');
    expect(payload.requests[0]?.body.kind).toBe('form');
    expect(payload.requests[0]?.body.form.map(row => row.key)).toEqual(['orgId', 'username', 'password']);
    expect(payload.requests[0]?.headers).toEqual([]);
  });

  it('uses bare authorization header and 1-based pagination', () => {
    const preview = parseDhyxManifest(MANIFEST_JSON);
    const payload = buildDhyxImportPayload(preview, new Set(['r:/system/org']));

    const page = payload.requests.find(request => request.external_key === '/system/org#GET');
    expect(page?.method).toBe('GET');
    expect(page?.url).toBe('/system/org');
    expect(page?.headers).toEqual([{ id: '', enabled: true, key: 'Authorization', value: '{{token}}' }]);
    expect(page?.query.map(row => `${row.key}=${row.value}`)).toEqual(['page=1', 'size=20', 'name=']);

    const tree = payload.requests.find(request => request.external_key === '/system/org/tree#GET');
    expect(tree?.name).toBe('机构管理控制器 · 获取机构树');
  });

  it('keeps path templates and wraps createAll body as array', () => {
    const preview = parseDhyxManifest(MANIFEST_JSON);
    const payload = buildDhyxImportPayload(preview, new Set(['r:/system/org']));

    const get = payload.requests.find(request => request.external_key === '/system/org/{id}#GET');
    expect(get?.url).toBe('/system/org/{id}');

    const create = payload.requests.find(request => request.external_key === '/system/org#POST');
    expect(JSON.parse(create?.body.text ?? 'null')).toEqual({ parentId: '', type: '' });

    const createAll = payload.requests.find(request => request.external_key === '/system/org/all#POST');
    expect(JSON.parse(createAll?.body.text ?? 'null')).toEqual([{ parentId: '', type: '' }]);
  });

  it('parses the real peis manifest if present', () => {
    let raw: string;
    try {
      raw = readFileSync('/tmp/peis-manifest.json', 'utf-8');
    } catch {
      return;
    }
    const preview = parseDhyxManifest(raw);
    expect(preview.totalRequests).toBeGreaterThan(1500);

    const keys = preview.modules
      .flatMap(module => module.resources)
      .flatMap(resource => resource.operations.map(operation => operation.externalKey));
    expect(new Set(keys).size).toBe(keys.length);

    const all = new Set(preview.modules.flatMap(module => module.resources.map(resource => resource.key)));
    const payload = buildDhyxImportPayload(preview, all);
    // peis 的 AuthController 自带真实登录端点，不再合成，总数即 totalRequests
    const hasLogin = preview.modules
      .flatMap(module => module.resources)
      .flatMap(resource => resource.operations)
      .some(operation => operation.externalKey === '/auth/oauth/token#POST');
    expect(hasLogin).toBe(true);
    expect(payload.requests).toHaveLength(preview.totalRequests);
    for (const request of payload.requests) {
      expect(request.url.startsWith('/')).toBe(true);
      expect(request.url).not.toContain('undefined');
    }
  });
});

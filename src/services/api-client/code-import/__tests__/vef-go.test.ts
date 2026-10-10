import { describe, expect, it } from 'vitest';

import { buildVefImportPayload, parseVefManifest } from '../vef-go';
import { SMP_MANIFEST_FIXTURE } from './vef-manifest.fixture';

const MANIFEST_JSON = JSON.stringify(SMP_MANIFEST_FIXTURE);

describe('parseVefManifest', () => {
  it('derives modules from resource names for business and framework resources', () => {
    const preview = parseVefManifest(MANIFEST_JSON);

    expect(preview.modules.map(module => module.name)).toEqual(['security', 'sys']);

    const sysModule = preview.modules.find(module => module.name === 'sys');
    expect(sysModule?.resources.map(resource => resource.name)).toEqual(['user']);
    expect(sysModule?.key).toBe('m:sys');

    const user = sysModule?.resources[0];
    expect(user?.fullName).toBe('smp/sys/user');
    expect(user?.key).toBe('r:smp/sys/user');
    expect(user?.operations.map(operation => operation.action)).toEqual([
      'change_password',
      'delete_many',
      'find_page',
    ]);
  });

  it('skips resources without operations and counts total requests', () => {
    const preview = parseVefManifest(MANIFEST_JSON);

    // smp/sys/empty 没有操作，不应出现
    const names = preview.modules.flatMap(module => module.resources.map(resource => resource.fullName));
    expect(names).not.toContain('smp/sys/empty');
    expect(preview.totalRequests).toBe(6);
  });

  it('builds stable external keys and param field tables', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const findPage = preview.modules
      .flatMap(module => module.resources)
      .flatMap(resource => resource.operations)
      .find(operation => operation.action === 'find_page');

    expect(findPage?.externalKey).toBe('smp/sys/user#find_page');
    expect(findPage?.permission).toBe('sys.user.query');
    expect(findPage?.paramsFields).toEqual([
      { name: 'keyword', type: 'string', required: false, label: null },
      { name: 'organizationId', type: 'string', required: false, label: '机构ID' },
      { name: 'phoneNumber', type: 'string', required: false, label: null },
    ]);
  });

  it('rejects invalid manifest json', () => {
    expect(() => parseVefManifest('not json')).toThrow('清单不是合法 JSON');
    expect(() => parseVefManifest('{"framework":"v"}')).toThrow('resources 数组');
  });
});

describe('buildVefImportPayload', () => {
  it('creates two-level groups only for selected resources', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const payload = buildVefImportPayload(preview, new Set(['r:smp/sys/user']));

    expect(payload.groups).toEqual([
      { key: 'm:sys', parent_key: null, name: 'sys' },
      { key: 'r:smp/sys/user', parent_key: 'm:sys', name: 'user' },
    ]);
    expect(payload.requests).toHaveLength(3);
  });

  it('emits rpc envelope with bearer header, params skeleton and pagination meta', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const payload = buildVefImportPayload(preview, new Set(['r:smp/sys/user']));

    const findPage = payload.requests.find(request => request.name === 'user · find_page');
    expect(findPage).toBeDefined();
    expect(findPage?.method).toBe('POST');
    expect(findPage?.url).toBe('/api');
    expect(findPage?.headers).toEqual([{ id: '', enabled: true, key: 'Authorization', value: 'Bearer {{token}}' }]);
    expect(findPage?.external_key).toBe('smp/sys/user#find_page');

    const body = JSON.parse(findPage?.body.text ?? '{}');
    expect(body.resource).toBe('smp/sys/user');
    expect(body.action).toBe('find_page');
    // 骨架放入字段表全部字段（含可选），按类型零值占位
    expect(body.params).toEqual({ keyword: '', organizationId: '', phoneNumber: '' });
    expect(body.meta).toEqual({ page: 1, size: 20 });
  });

  it('maps scalar placeholders by field type', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const payload = buildVefImportPayload(preview, new Set(['r:smp/sys/user']));

    const changePassword = payload.requests.find(request => request.name === 'user · change_password');
    expect(JSON.parse(changePassword?.body.text ?? '{}').params).toEqual({
      oldPassword: '',
      newPassword: '',
      confirmPassword: '',
    });

    const deleteMany = payload.requests.find(request => request.name === 'user · delete_many');
    expect(JSON.parse(deleteMany?.body.text ?? '{}').params).toEqual({ pks: [] });
  });

  it('omits auth header and pagination meta for public login action', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const payload = buildVefImportPayload(preview, new Set(['r:security/auth']));

    const login = payload.requests.find(request => request.name === 'auth · login');
    expect(login?.headers).toEqual([]);
    expect(login?.method).toBe('POST');

    const body = JSON.parse(login?.body.text ?? '{}');
    expect(body.resource).toBe('security/auth');
    expect(body.params).toEqual({ type: '', principal: '', credentials: null });
    expect(body.meta).toBeUndefined();

    // 同一资源下的 bearer 操作仍然带头
    const getUserInfo = payload.requests.find(request => request.name === 'auth · get_user_info');
    expect(getUserInfo?.headers[0]?.key).toBe('Authorization');
  });

  it('selects multiple resources across modules', () => {
    const preview = parseVefManifest(MANIFEST_JSON);
    const payload = buildVefImportPayload(preview, new Set(['r:smp/sys/user', 'r:security/auth']));

    expect(payload.groups.map(group => group.key)).toEqual([
      'm:security',
      'r:security/auth',
      'm:sys',
      'r:smp/sys/user',
    ]);
    expect(payload.requests).toHaveLength(6);
  });
});

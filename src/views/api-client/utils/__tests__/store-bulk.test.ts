import { strict as assert } from 'node:assert';

import { createPinia, defineStore, setActivePinia } from 'pinia';
import { beforeEach, describe, it } from 'vitest';

import { createApiClientStore } from '@/stores/api-client';
import type { ApiClientService } from '@/stores/api-client';
import type { ApiClientGroup, ApiClientRequest } from '@/types/api-client';
import { buildGroupTree, flattenGroupTree } from '@/views/api-client-workspace/components/build-group-tree';

/**
 * 大数据量回归：按 vef-java 导入的真实规模（13 模块 + 136 资源两级分组、
 * 2110 条请求）验证 selectProject 与树构建逻辑。
 */
describe('api client store with bulk imported data', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('loads two-level groups and bulk requests through selectProject', async () => {
    const groups: ApiClientGroup[] = [];
    let groupSeq = 0;
    for (let moduleIndex = 0; moduleIndex < 13; moduleIndex++) {
      const moduleGroup: ApiClientGroup = {
        id: `mod_${moduleIndex}`,
        projectId: 'p1',
        parentGroupId: null,
        name: `module${moduleIndex}`,
        sort: moduleIndex,
        createdAt: 0,
        updatedAt: 0,
      };
      groups.push(moduleGroup);
      for (let resIndex = 0; resIndex < 10; resIndex++) {
        groups.push({
          id: `res_${groupSeq++}`,
          projectId: 'p1',
          parentGroupId: moduleGroup.id,
          name: `resource-${moduleIndex}-${resIndex}`,
          sort: resIndex,
          createdAt: 0,
          updatedAt: 0,
        });
      }
    }

    const requests: ApiClientRequest[] = Array.from({ length: 2110 }, (_, index) => ({
      id: `req_${index}`,
      groupId: groups[13 + (index % 130)]?.id ?? '',
      projectId: 'p1',
      name: `res ${index} · action`,
      method: 'GET',
      url: '/system/x',
      query: [{ id: 'q1', enabled: true, key: 'page', value: '0' }],
      headers: [{ id: 'h1', enabled: true, key: 'Authorization', value: 'Bearer {{token}}' }],
      body: { kind: index % 3 === 0 ? 'none' : 'json', text: index % 3 === 0 ? '' : '{"a":1}', form: [] },
      timeoutMs: 30000,
      aiPrompt: '',
      aiReference: '',
      externalKey: `/x/${index}#GET`,
      sort: index,
      createdAt: 0,
      updatedAt: 0,
    }));

    const service = {
      listApiGroups: async () => groups,
      listApiRequests: async () => requests,
      listApiEnvironments: async () => [],
    } as unknown as ApiClientService;

    const useStore = defineStore('api-client-bulk', () => createApiClientStore({ service }));
    const store = useStore();

    await store.selectProject('p1', { discardUnsaved: true });

    assert.equal(store.groups.length, 143);
    assert.equal(store.requests.length, 2110);
    assert.equal(store.requestSearchResults.length, 2110);

    const roots = buildGroupTree(store.groups);
    assert.equal(roots.length, 13);
    const flattened = flattenGroupTree(roots);
    assert.equal(flattened.length, 143);
    assert.equal(flattened[0]?.depth, 1);
    assert.equal(flattened[1]?.depth, 2);
  });
});

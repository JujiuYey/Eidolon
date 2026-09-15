import { strict as assert } from 'node:assert';
import { describe, it } from 'vitest';
import type { ApiClientGroup } from '@/types/api-client';
import { buildGroupTree, flattenGroupTree } from '../build-group-tree';

function makeGroup(overrides: Partial<ApiClientGroup>): ApiClientGroup {
  return {
    id: overrides.id ?? 'g',
    projectId: 'project-1',
    parentGroupId: null,
    name: 'g',
    sort: 0,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

describe('buildGroupTree', () => {
  it('returns flat list of roots when there are no children', () => {
    const groups: ApiClientGroup[] = [
      makeGroup({ id: 'a', sort: 2 }),
      makeGroup({ id: 'b', sort: 1 }),
    ];

    const tree = buildGroupTree(groups);

    assert.deepEqual(tree.map(node => node.group.id), ['b', 'a']);
    for (const node of tree) {
      assert.equal(node.depth, 1);
      assert.deepEqual(node.children, []);
    }
  });

  it('nests children under their parent and propagates depth', () => {
    const groups: ApiClientGroup[] = [
      makeGroup({ id: 'root', sort: 0 }),
      makeGroup({ id: 'child', parentGroupId: 'root', sort: 0 }),
      makeGroup({ id: 'grandchild', parentGroupId: 'child', sort: 0 }),
    ];

    const tree = buildGroupTree(groups);
    const flattened = flattenGroupTree(tree);

    assert.deepEqual(
      flattened.map(node => [node.group.id, node.depth]),
      [
        ['root', 1],
        ['child', 2],
        ['grandchild', 3],
      ],
    );
  });

  it('siblings under the same parent are sorted by sort ascending', () => {
    const groups: ApiClientGroup[] = [
      makeGroup({ id: 'root', sort: 0 }),
      makeGroup({ id: 'b', parentGroupId: 'root', sort: 2 }),
      makeGroup({ id: 'a', parentGroupId: 'root', sort: 1 }),
    ];

    const tree = buildGroupTree(groups);
    const root = tree[0];

    assert.ok(root);
    assert.deepEqual(root?.children.map(node => node.group.id), ['a', 'b']);
  });

  it('surfaces orphan children (parent missing) as roots so they remain reachable', () => {
    const groups: ApiClientGroup[] = [
      makeGroup({ id: 'orphan', parentGroupId: 'missing-parent', sort: 0 }),
      makeGroup({ id: 'root', sort: 0 }),
    ];

    const tree = buildGroupTree(groups);

    assert.deepEqual(tree.map(node => node.group.id), ['root', 'orphan']);
  });
});

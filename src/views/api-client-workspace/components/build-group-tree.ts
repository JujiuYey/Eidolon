import type { ApiClientGroup } from '@/types/api-client';

export interface GroupTreeNode {
  group: ApiClientGroup;
  depth: number;
  children: GroupTreeNode[];
}

/**
 * Builds a hierarchical tree of groups. Roots have `parentGroupId === null`;
 * children are resolved by id. Siblings are sorted by `sort` ascending.
 *
 * Orphans (children whose parent is missing from the input) are surfaced as
 * roots so they remain reachable in the UI instead of vanishing silently.
 */
export function buildGroupTree(groups: ApiClientGroup[]): GroupTreeNode[] {
  const knownIds = new Set<string>();
  for (const group of groups) {
    knownIds.add(group.id);
  }

  const childrenByParent = new Map<string, ApiClientGroup[]>();
  for (const group of groups) {
    const key = group.parentGroupId ?? '';
    const list = childrenByParent.get(key) ?? [];
    list.push(group);
    childrenByParent.set(key, list);
  }
  for (const list of childrenByParent.values()) {
    list.sort((a, b) => a.sort - b.sort);
  }

  function build(parentId: string | null, depth: number): GroupTreeNode[] {
    const siblings = childrenByParent.get(parentId ?? '') ?? [];
    const nodes: GroupTreeNode[] = [];
    for (const group of siblings) {
      const children = build(group.id, depth + 1);
      nodes.push({ group, depth, children });
    }
    return nodes;
  }

  const roots = build(null, 1);

  const orphanParents: string[] = [];
  for (const parentId of childrenByParent.keys()) {
    if (parentId !== '' && !knownIds.has(parentId)) {
      orphanParents.push(parentId);
    }
  }
  for (const parentId of orphanParents) {
    const siblings = childrenByParent.get(parentId);
    if (!siblings) {
      continue;
    }
    for (const group of siblings) {
      const children = build(group.id, 2);
      roots.push({ group, depth: 1, children });
    }
  }

  return roots;
}

/**
 * Flattens a tree built by `buildGroupTree` into DFS pre-order so the UI can
 * render it with a single `v-for`. Each entry carries its depth for indentation.
 */
export function flattenGroupTree(roots: GroupTreeNode[]): GroupTreeNode[] {
  const out: GroupTreeNode[] = [];
  function walk(nodes: GroupTreeNode[]): void {
    for (const node of nodes) {
      out.push(node);
      if (node.children.length > 0) {
        walk(node.children);
      }
    }
  }
  walk(roots);
  return out;
}

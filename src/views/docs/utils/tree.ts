import type { DocsEntry } from '@/types/docs';

export interface DocsTreeNode {
  entry: DocsEntry;
  children: DocsTreeNode[];
}

export interface VisibleDocsRow {
  entry: DocsEntry;
  depth: number;
}

function parentPathOf(path: string): string {
  const index = path.lastIndexOf('/');
  return index === -1 ? '' : path.slice(0, index);
}

/**
 * 扁平条目列表 → 树。后端列表已是「目录在前、同层按名称」排序，
 * 逐个 append 即可保持每层的相对顺序；父目录缺失的孤儿节点挂到根，
 * 避免整棵子树凭空消失。
 */
export function buildDocsTree(entries: DocsEntry[]): DocsTreeNode[] {
  const nodes = new Map<string, DocsTreeNode>();
  for (const entry of entries) {
    nodes.set(entry.path, { entry, children: [] });
  }

  const roots: DocsTreeNode[] = [];
  for (const entry of entries) {
    const node = nodes.get(entry.path);
    if (!node) {
      continue;
    }
    const parentPath = parentPathOf(entry.path);
    const parent = parentPath ? nodes.get(parentPath) : undefined;
    if (parent) {
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  return roots;
}

/** 根层目录路径，用于初始化默认展开集合 */
export function firstLevelDirPaths(entries: DocsEntry[]): string[] {
  return entries
    .filter(entry => entry.is_dir && !entry.path.includes('/'))
    .map(entry => entry.path);
}

/** 按展开集合把树拍平成带缩进深度的可见行，供单层 v-for 渲染 */
export function flattenTree(nodes: DocsTreeNode[], expanded: Set<string>): VisibleDocsRow[] {
  const rows: VisibleDocsRow[] = [];

  const walk = (list: DocsTreeNode[], depth: number) => {
    for (const node of list) {
      rows.push({ entry: node.entry, depth });
      if (node.entry.is_dir && expanded.has(node.entry.path)) {
        walk(node.children, depth + 1);
      }
    }
  };

  walk(nodes, 0);
  return rows;
}

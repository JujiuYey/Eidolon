import { describe, expect, it } from 'vitest';
import type { DocsEntry } from '@/types/docs';
import { buildDocsTree, firstLevelDirPaths, flattenTree } from '../tree';

function makeEntry(overrides: Partial<DocsEntry> & Pick<DocsEntry, 'name' | 'path' | 'is_dir'>): DocsEntry {
  return { size: 10, updated_at: 1, ...overrides };
}

// 模拟后端排序：目录在前，同层按名称
const entries: DocsEntry[] = [
  makeEntry({ name: '周报', path: '周报', is_dir: true }),
  makeEntry({ name: '笔记', path: '笔记', is_dir: true }),
  makeEntry({ name: '2026-10.md', path: '周报/2026-10.md', is_dir: false }),
  makeEntry({ name: '子目录', path: '周报/子目录', is_dir: true }),
  makeEntry({ name: 'deep.md', path: '周报/子目录/deep.md', is_dir: false }),
  makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false }),
  makeEntry({ name: '灵感.md', path: '笔记/灵感.md', is_dir: false }),
];

describe('buildDocsTree', () => {
  it('按路径父子关系建树并保持同层顺序', () => {
    const tree = buildDocsTree(entries);

    expect(tree.map(node => node.entry.name)).toEqual(['周报', '笔记', 'readme.md']);

    const weekly = tree[0];
    expect(weekly?.children.map(node => node.entry.name)).toEqual(['2026-10.md', '子目录']);
    expect(weekly?.children[1]?.children[0]?.entry.path).toBe('周报/子目录/deep.md');
  });

  it('父目录缺失的孤儿节点挂到根而不是消失', () => {
    const orphan = makeEntry({ name: '孤儿.md', path: '不存在/孤儿.md', is_dir: false });
    const tree = buildDocsTree([...entries, orphan]);

    expect(tree.some(node => node.entry.path === '不存在/孤儿.md')).toBe(true);
  });
});

describe('firstLevelDirPaths', () => {
  it('只返回根层目录', () => {
    expect(firstLevelDirPaths(entries)).toEqual(['周报', '笔记']);
  });
});

describe('flattenTree', () => {
  it('折叠时只显示根层，展开后按深度缩进', () => {
    const tree = buildDocsTree(entries);

    const collapsed = flattenTree(tree, new Set());
    expect(collapsed.map(row => row.entry.path)).toEqual(['周报', '笔记', 'readme.md']);
    expect(collapsed.every(row => row.depth === 0)).toBe(true);

    const expanded = flattenTree(tree, new Set(['周报', '周报/子目录']));
    expect(expanded.map(row => row.entry.path)).toEqual([
      '周报',
      '周报/2026-10.md',
      '周报/子目录',
      '周报/子目录/deep.md',
      '笔记',
      'readme.md',
    ]);
    expect(expanded.find(row => row.entry.path === '周报/子目录/deep.md')?.depth).toBe(2);
  });
});

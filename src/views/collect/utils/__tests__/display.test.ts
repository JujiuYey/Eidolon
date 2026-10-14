import { describe, expect, it } from 'vitest';

import type { CollectItem } from '@/types/collect';

import {
  filterCounts,
  filterItems,
  formatCreatedAt,
  hostOf,
  isLinkText,
  remainingDays,
  sourceOf,
  sourceOptions,
  topCollectHosts,
  TRASH_RETENTION_DAYS,
} from '../display';

const NOW = new Date(2026, 9, 14, 15).getTime(); // 周三下午
const DAY = 24 * 60 * 60 * 1000;

function makeItem(overrides: Partial<CollectItem> = {}): CollectItem {
  return {
    id: 'collect_1',
    kind: 'note',
    url: null,
    title: null,
    description: null,
    content: '随手记',
    favicon_url: null,
    archived: false,
    created_at: 100,
    updated_at: 100,
    deleted_at: null,
    ...overrides,
  };
}

describe('collect display helpers', () => {
  it('classifies link text by http(s) prefix', () => {
    expect(isLinkText('https://example.com/a?b=1')).toBe(true);
    expect(isLinkText('http://example.com')).toBe(true);
    expect(isLinkText('  https://example.com  ')).toBe(true);
    expect(isLinkText('ftp://example.com')).toBe(false);
    expect(isLinkText('看看这个 https://example.com')).toBe(false);
    expect(isLinkText('随手记点什么')).toBe(false);
    expect(isLinkText('')).toBe(false);
  });

  it('extracts host from url and tolerates invalid urls', () => {
    expect(hostOf('https://example.com/a/b?q=1')).toBe('example.com');
    expect(hostOf('http://192.168.1.2:8080/x')).toBe('192.168.1.2:8080');
    expect(hostOf('not a url')).toBe('not a url');
    expect(hostOf(null)).toBe('');
  });

  it('counts items per filter with archived excluded from active counts', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link' }),
      makeItem({ id: 'b', kind: 'note' }),
      makeItem({ id: 'c', kind: 'link', archived: true }),
    ];

    expect(filterCounts(items)).toEqual({ all: 2, link: 1, note: 1, archived: 1 });
  });

  it('filters by kind and archived, then by keyword across searchable fields', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link', title: 'Rust 文章', url: 'https://rust-lang.org' }),
      makeItem({ id: 'b', kind: 'note', content: '周五交周报' }),
      makeItem({ id: 'c', kind: 'link', title: '已归档的链接', archived: true }),
    ];

    expect(filterItems(items, 'all', '').map(item => item.id)).toEqual(['a', 'b']);
    expect(filterItems(items, 'link', '').map(item => item.id)).toEqual(['a']);
    expect(filterItems(items, 'archived', '').map(item => item.id)).toEqual(['c']);

    // 关键字命中标题（大小写不敏感）
    expect(filterItems(items, 'all', 'rust').map(item => item.id)).toEqual(['a']);
    // 关键字命中正文
    expect(filterItems(items, 'note', '周报').map(item => item.id)).toEqual(['b']);
    // 关键字命中 URL
    expect(filterItems(items, 'all', 'rust-lang.org')).toEqual([items[0]]);
    // 归档项不参与关键字搜索（除归档页外）
    expect(filterItems(items, 'all', '已归档的链接')).toEqual([]);
    expect(filterItems(items, 'archived', '已归档的链接').map(item => item.id)).toEqual(['c']);
  });

  it('formats timestamps as local ISO date and blanks zero', () => {
    expect(formatCreatedAt(0)).toBe('');
    expect(formatCreatedAt(new Date(2026, 9, 12, 8, 30).getTime())).toBe('2026-10-12');
  });

  it('sourceOf normalizes hosts: lowercase, strips www, http(s) only', () => {
    expect(sourceOf('https://www.GitHub.com/a/b')).toBe('github.com');
    expect(sourceOf('http://GitHub.com')).toBe('github.com');
    expect(sourceOf('https://192.168.1.2:8080/x')).toBe('192.168.1.2:8080');
    expect(sourceOf('ftp://files.com/x')).toBe('');
    expect(sourceOf('not a url')).toBe('');
    expect(sourceOf(null)).toBe('');
  });

  it('sourceOptions counts non-archived links only, sorted by count desc', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link', url: 'https://www.github.com/1' }),
      makeItem({ id: 'b', kind: 'link', url: 'https://github.com/2' }),
      makeItem({ id: 'c', kind: 'link', url: 'https://example.com/3' }),
      makeItem({ id: 'd', kind: 'note', content: '正文提到 github.com 也不算' }),
      makeItem({ id: 'e', kind: 'link', url: 'https://archived.com/4', archived: true }),
    ];

    expect(sourceOptions(items)).toEqual([
      { host: 'github.com', count: 2 },
      { host: 'example.com', count: 1 },
    ]);
  });

  it('filters by exact source on top of kind/keyword, and in archived view', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link', url: 'https://www.github.com/1', title: '仓库' }),
      makeItem({ id: 'b', kind: 'link', url: 'https://github.com/2', title: '文档' }),
      makeItem({ id: 'c', kind: 'link', url: 'https://example.com/3' }),
      makeItem({ id: 'd', kind: 'note', content: '记事没有来源' }),
      makeItem({ id: 'e', kind: 'link', url: 'https://github.com/old', archived: true }),
    ];

    // www 与裸域名归一到同一来源
    expect(filterItems(items, 'all', '', 'github.com').map(item => item.id)).toEqual(['a', 'b']);
    // 来源与关键字叠加
    expect(filterItems(items, 'all', '文档', 'github.com').map(item => item.id)).toEqual(['b']);
    // 归档视图里来源同样生效
    expect(filterItems(items, 'archived', '', 'github.com').map(item => item.id)).toEqual(['e']);
    // 记事不匹配任何来源
    expect(filterItems(items, 'note', '', 'github.com')).toEqual([]);
  });

  it('topCollectHosts ranks link hosts by count and keeps archived items', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link', url: 'https://rust-lang.org/a', created_at: NOW }),
      makeItem({ id: 'b', kind: 'link', url: 'https://rust-lang.org/b', created_at: NOW - 1 * DAY }),
      makeItem({ id: 'c', kind: 'link', url: 'https://example.com/c', created_at: NOW - 2 * DAY }),
      makeItem({ id: 'd', kind: 'note', content: '记事', created_at: NOW - 3 * DAY }),
      makeItem({ id: 'e', kind: 'link', url: 'https://old.com/e', created_at: NOW - 40 * DAY, archived: true }),
    ];

    // 归档条目也参与（收藏过就算）；同数按域名排序
    expect(topCollectHosts(items)).toEqual([
      { host: 'rust-lang.org', count: 2 },
      { host: 'example.com', count: 1 },
      { host: 'old.com', count: 1 },
    ]);
  });

  it('topCollectHosts applies limit and skips non-link or unparsable urls', () => {
    const items = [
      makeItem({ id: 'a', kind: 'note', url: null }),
      makeItem({ id: 'b', kind: 'link', url: 'not-a-url' }),
      makeItem({ id: 'c', kind: 'link', url: 'ftp://files.com/x' }),
      makeItem({ id: 'd', kind: 'link', url: 'https://a.com/1' }),
      makeItem({ id: 'e', kind: 'link', url: 'https://b.com/1' }),
      makeItem({ id: 'f', kind: 'link', url: 'https://c.com/1' }),
    ];

    expect(topCollectHosts(items, 2)).toEqual([
      { host: 'a.com', count: 1 },
      { host: 'b.com', count: 1 },
    ]);
  });
});

describe('trash retention helpers', () => {
  it('remainingDays counts down from 30 by elapsed days', () => {
    const deletedAt = NOW;
    expect(remainingDays(deletedAt, NOW)).toBe(30);
    expect(remainingDays(deletedAt, NOW + 1 * DAY)).toBe(29);
    expect(remainingDays(deletedAt, NOW + 29 * DAY + 3600_000)).toBe(1);
    // 过期后钳到 0
    expect(remainingDays(deletedAt, NOW + 40 * DAY)).toBe(0);
  });

  it('remainingDays treats missing deleted_at as full retention', () => {
    expect(remainingDays(null, NOW)).toBe(TRASH_RETENTION_DAYS);
  });
});

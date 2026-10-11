import { describe, expect, it } from 'vitest';

import type { CollectItem } from '@/types/collect';

import { filterCounts, filterItems, formatCreatedAt, hostOf, isLinkText } from '../display';

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
});

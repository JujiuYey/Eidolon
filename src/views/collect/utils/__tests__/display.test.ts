import { describe, expect, it } from 'vitest';

import type { CollectItem } from '@/types/collect';

import { computeCollectStats, filterCounts, filterItems, formatCreatedAt, hostOf, isLinkText } from '../display';

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

  it('computes collect stats: week/month counts, streak, archive, trend and top hosts', () => {
    const items = [
      makeItem({ id: 'a', kind: 'link', url: 'https://rust-lang.org/a', created_at: NOW }),
      makeItem({ id: 'b', kind: 'link', url: 'https://rust-lang.org/b', created_at: NOW - 1 * DAY }),
      makeItem({ id: 'c', kind: 'link', url: 'https://example.com/c', created_at: NOW - 2 * DAY }),
      makeItem({ id: 'd', kind: 'note', content: '记事', created_at: NOW - 3 * DAY }),
      makeItem({ id: 'e', kind: 'link', url: 'https://old.com/e', created_at: NOW - 40 * DAY, archived: true }),
    ];

    const stats = computeCollectStats(items, NOW);

    expect(stats.weekCount).toBe(3); // 周一以后：今天、昨天、前天（周一起）
    expect(stats.monthCount).toBe(4); // 30 天窗口内的 4 条（40 天前那条在外）
    expect(stats.streakDays).toBe(4); // 今天到 3 天前连续
    expect(stats.archivedCount).toBe(1);
    expect(stats.dailyCounts).toHaveLength(30);
    expect(stats.dailyCounts[29]).toBe(1);
    expect(stats.dailyTooltips[29]).toContain('1 条收藏');
    // 归档条目也参与域名统计（收藏过就算）
    expect(stats.topHosts).toEqual([
      { host: 'rust-lang.org', count: 2 },
      { host: 'example.com', count: 1 },
      { host: 'old.com', count: 1 },
    ]);
  });

  it('computeCollectStats breaks streak when a day is missing and tolerates bad urls', () => {
    const items = [
      makeItem({ id: 'a', created_at: NOW }),
      makeItem({ id: 'b', created_at: NOW - 3 * DAY }),
      makeItem({ id: 'c', kind: 'link', url: 'not-a-url', created_at: NOW }),
      makeItem({ id: 'd', kind: 'link', url: 'ftp://files.com/x', created_at: NOW }),
    ];

    const stats = computeCollectStats(items, NOW);

    expect(stats.streakDays).toBe(1); // 昨天、前天空缺
    expect(stats.topHosts).toEqual([]); // 非法/非 http(s) URL 不参与域名统计
  });
});

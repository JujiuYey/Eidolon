import { describe, expect, it } from 'vitest';

import type { DocsEntry } from '@/types/docs';

import { computeDocsStats, lastUpdatedHint, rootDocStats } from '../docs-stats';

const NOW = new Date(2026, 9, 14, 15).getTime();
const DAY = 24 * 60 * 60 * 1000;

function makeEntry(overrides: Partial<DocsEntry> = {}): DocsEntry {
  return {
    name: '笔记.md',
    path: '笔记.md',
    is_dir: false,
    size: 100,
    updated_at: NOW,
    ...overrides,
  };
}

describe('docs stats', () => {
  it('counts docs across roots, ignoring directories and out-of-window updates', () => {
    const stats = computeDocsStats(
      2,
      [
        [
          makeEntry({ updated_at: NOW }),
          makeEntry({ name: '旧.md', path: '旧.md', updated_at: NOW - 40 * DAY }),
          makeEntry({ name: '子目录', path: '子目录', is_dir: true, updated_at: NOW }),
        ],
        [makeEntry({ name: '另一库.md', path: '另一库.md', updated_at: NOW - 2 * DAY })],
      ],
      NOW,
    );

    expect(stats.rootCount).toBe(2);
    expect(stats.docCount).toBe(3); // 目录不算
    expect(stats.recentUpdatedCount).toBe(2); // 40 天前那份在窗口外
    expect(stats.lastUpdatedAt).toBe(NOW);
    expect(stats.dailyUpdated).toHaveLength(30);
    expect(stats.dailyUpdated[29]).toBe(1);
    expect(stats.dailyUpdated[27]).toBe(1);
    expect(stats.dailyTooltips[29]).toContain('1 篇更新');
  });

  it('handles empty roots and docs', () => {
    const stats = computeDocsStats(1, [[]], NOW);
    expect(stats.docCount).toBe(0);
    expect(stats.recentUpdatedCount).toBe(0);
    expect(stats.lastUpdatedAt).toBe(0);
    expect(lastUpdatedHint(stats.lastUpdatedAt)).toBe('');
  });

  it('lastUpdatedHint renders the short date', () => {
    expect(lastUpdatedHint(NOW)).toBe('最近 10-14');
  });

  it('counts docs per root keeping labels aligned, files only', () => {
    const counts = rootDocStats(
      ['工作笔记', '资料库'],
      [
        [
          makeEntry(),
          makeEntry({ name: '旧.md', path: '旧.md' }),
          makeEntry({ name: '子目录', path: '子目录', is_dir: true }),
        ],
        [makeEntry({ name: '另一库.md', path: '另一库.md' })],
      ],
    );

    expect(counts).toEqual([
      { label: '工作笔记', value: 2 },
      { label: '资料库', value: 1 },
    ]);
  });
});

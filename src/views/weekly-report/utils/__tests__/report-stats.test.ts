import { describe, expect, it } from 'vitest';

import type { WeeklyReportEntry, WeeklyReportRepo } from '@/types/weekly-report';

import { computeWeeklyReportStats, lastReportHint } from '../report-stats';

const NOW = new Date(2026, 9, 14, 15).getTime();
const DAY = 24 * 60 * 60 * 1000;

function makeRepo(overrides: Partial<WeeklyReportRepo> = {}): WeeklyReportRepo {
  return {
    id: 'repo_1',
    path: '/repos/api',
    name: 'api',
    sort: 0,
    created_at: NOW - 30 * DAY,
    updated_at: NOW - 30 * DAY,
    ...overrides,
  };
}

function makeReport(overrides: Partial<WeeklyReportEntry> = {}): WeeklyReportEntry {
  return {
    id: 'report_1',
    title: '第 41 周',
    range_start: NOW - 7 * DAY,
    range_end: NOW,
    content: '# 周报',
    repo_count: 1,
    commit_count: 12,
    created_at: NOW,
    updated_at: NOW,
    ...overrides,
  };
}

describe('weekly report stats', () => {
  it('counts repos, reports, commits and this-month reports', () => {
    const stats = computeWeeklyReportStats(
      [
        makeReport({ commit_count: 12 }),
        makeReport({ id: 'report_2', commit_count: 8, created_at: NOW - 2 * DAY }),
        makeReport({ id: 'report_3', commit_count: 30, created_at: NOW - 40 * DAY }),
      ],
      [makeRepo(), makeRepo({ id: 'repo_2', name: 'web' })],
      NOW,
    );

    expect(stats.repoCount).toBe(2);
    expect(stats.reportCount).toBe(3);
    expect(stats.totalCommitCount).toBe(50);
    expect(stats.monthReportCount).toBe(2); // 40 天前那份在窗口外
    expect(stats.lastReportAt).toBe(NOW);
  });

  it('handles empty state', () => {
    const stats = computeWeeklyReportStats([], [], NOW);
    expect(stats.reportCount).toBe(0);
    expect(stats.totalCommitCount).toBe(0);
    expect(stats.lastReportAt).toBe(0);
    expect(lastReportHint(stats.lastReportAt)).toBe('');
  });

  it('lastReportHint renders the short date', () => {
    expect(lastReportHint(NOW)).toBe('最近 10-14');
  });
});

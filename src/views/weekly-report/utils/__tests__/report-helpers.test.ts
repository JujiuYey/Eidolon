import { describe, expect, it } from 'vitest';

import type { RepoCommitLog } from '@/types/weekly-report';

import {
  aggregateCommits,
  buildReportTitle,
  collectCommits,
  formatDate,
  formatDateTime,
  formatRange,
  getDaysRange,
  getLastWeekRange,
  getThisWeekRange,
  parseCommitSubject,
} from '../report-helpers';

/** 2026-10-04 是周日；本周一为 2026-09-28 */
const SUNDAY = new Date(2026, 9, 4, 15, 30);

function log(
  repoName: string,
  commits: Array<{ hash: string; subject: string; timestamp?: number }>,
  error: string | null = null,
): RepoCommitLog {
  return {
    repo_id: `id-${repoName}`,
    repo_name: repoName,
    error,
    commits: commits.map((commit, index) => ({
      hash: commit.hash,
      short_hash: commit.hash.slice(0, 7),
      subject: commit.subject,
      author_name: '我',
      timestamp: commit.timestamp ?? 1_790_700_000_000 + index,
    })),
  };
}

describe('date ranges', () => {
  it('this week starts on Monday and ends on Sunday', () => {
    const range = getThisWeekRange(SUNDAY);

    expect(new Date(range.start).getDay()).toBe(1);
    expect(formatDate(range.start)).toBe('2026.09.28');
    expect(formatDate(range.end)).toBe('2026.10.04');
    expect(new Date(range.start).getHours()).toBe(0);
    expect(new Date(range.end).getHours()).toBe(23);
  });

  it('this week works when today is Monday', () => {
    const monday = new Date(2026, 8, 28, 9, 0);
    const range = getThisWeekRange(monday);

    expect(formatDate(range.start)).toBe('2026.09.28');
    expect(formatDate(range.end)).toBe('2026.10.04');
  });

  it('last week is shifted exactly seven days back', () => {
    const thisWeek = getThisWeekRange(SUNDAY);
    const lastWeek = getLastWeekRange(SUNDAY);

    expect(thisWeek.end - lastWeek.end).toBe(7 * 24 * 60 * 60 * 1000);
    expect(formatDate(lastWeek.start)).toBe('2026.09.21');
  });

  it('days range includes today', () => {
    const range = getDaysRange(7, SUNDAY);

    expect(formatDate(range.start)).toBe('2026.09.28');
    expect(formatDate(range.end)).toBe('2026.10.04');
  });

  it('formats ranges, datetimes and titles', () => {
    const range = getThisWeekRange(SUNDAY);

    expect(formatRange(range)).toBe('2026.09.28 - 2026.10.04');
    expect(buildReportTitle(range)).toBe('周报 2026.09.28 - 2026.10.04');
    expect(formatDateTime(new Date(2026, 9, 4, 9, 5).getTime())).toBe('2026-10-04 09:05');
  });
});

describe('parseCommitSubject', () => {
  it('parses conventional commits with scope', () => {
    expect(parseCommitSubject('feat(mail): 支持模板变量')).toEqual({
      type: 'feat',
      scope: 'mail',
      text: '支持模板变量',
    });
  });

  it('parses conventional commits without scope and handles breaking mark', () => {
    expect(parseCommitSubject('fix!: 修复空指针')).toEqual({
      type: 'fix',
      scope: '',
      text: '修复空指针',
    });
  });

  it('lowercases types and trims text', () => {
    expect(parseCommitSubject('  FEAT(api) : 新接口 ')).toEqual({
      type: 'feat',
      scope: 'api',
      text: '新接口',
    });
  });

  it('returns the raw subject for non-conventional commits', () => {
    expect(parseCommitSubject('更新依赖')).toEqual({
      type: '',
      scope: '',
      text: '更新依赖',
    });
  });
});

describe('collectCommits', () => {
  it('dedupes by hash, sorts by time and counts successful repos', () => {
    const collected = collectCommits([
      log('beta', [{ hash: 'b2', subject: 'fix: 两个', timestamp: 200 }]),
      log('alpha', [
        { hash: 'a1', subject: 'feat: 一个', timestamp: 300 },
        { hash: 'b2', subject: 'fix: 两个', timestamp: 200 },
      ]),
    ]);

    expect(collected.repo_count).toBe(2);
    expect(collected.errors).toEqual([]);
    expect(collected.commits.map(commit => commit.hash)).toEqual(['b2', 'a1']);
    expect(collected.commits[0]?.repo_name).toBe('beta');
  });

  it('collects failed repos into errors without dropping successes', () => {
    const collected = collectCommits([
      log('alpha', [{ hash: 'a1', subject: 'feat: 一个' }]),
      log('broken', [], 'git log 失败: not a repository'),
    ]);

    expect(collected.repo_count).toBe(1);
    expect(collected.commits).toHaveLength(1);
    expect(collected.errors).toEqual([
      { repo_name: 'broken', error: 'git log 失败: not a repository' },
    ]);
  });
});

describe('aggregateCommits', () => {
  const range = getThisWeekRange(SUNDAY);

  it('returns empty string when nothing was collected', () => {
    expect(aggregateCommits([], { range, groupBy: 'type' })).toBe('');
  });

  it('groups by conventional type with stable section order', () => {
    const markdown = aggregateCommits(
      [
        log('Eidolon', [
          { hash: '1', subject: 'chore: 升级依赖' },
          { hash: '2', subject: 'feat(mail): 支持模板变量' },
          { hash: '3', subject: 'fix(api): 修复空指针' },
          { hash: '4', subject: '随手改改' },
        ]),
      ],
      { range, groupBy: 'type' },
    );

    expect(markdown).toContain('# 周报 2026.09.28 - 2026.10.04');
    expect(markdown).toContain('1 个仓库 ｜ 4 次提交');

    const featIndex = markdown.indexOf('## 新功能');
    const fixIndex = markdown.indexOf('## 缺陷修复');
    const choreIndex = markdown.indexOf('## 其他');
    expect(featIndex).toBeGreaterThan(-1);
    expect(fixIndex).toBeGreaterThan(featIndex);
    expect(choreIndex).toBeGreaterThan(fixIndex);

    expect(markdown).toContain('- 支持模板变量（mail · Eidolon）');
    expect(markdown).toContain('- 修复空指针（api · Eidolon）');
    expect(markdown).toContain('- 随手改改（Eidolon）');
    expect(markdown).toContain('- 升级依赖（Eidolon）');
  });

  it('groups by repo keeping the type prefix', () => {
    const markdown = aggregateCommits(
      [
        log('Eidolon', [{ hash: '1', subject: 'feat(mail): 支持模板变量' }]),
        log('Nebula', [{ hash: '2', subject: '直接描述' }]),
      ],
      { range, groupBy: 'repo' },
    );

    expect(markdown).toContain('## Eidolon');
    expect(markdown).toContain('- feat(mail): 支持模板变量');
    expect(markdown).toContain('## Nebula');
    expect(markdown).toContain('- 直接描述');
    expect(markdown).not.toContain('·');
  });

  it('appends failed repos as a warning block', () => {
    const markdown = aggregateCommits(
      [
        log('Eidolon', [{ hash: '1', subject: 'feat: 新功能' }]),
        log('broken', [], '目录不是 git 仓库'),
      ],
      { range, groupBy: 'type' },
    );

    expect(markdown).toContain('⚠️ 以下仓库拉取失败');
    expect(markdown).toContain('- broken：目录不是 git 仓库');
    expect(markdown).toContain('1 个仓库 ｜ 1 次提交');
  });
});

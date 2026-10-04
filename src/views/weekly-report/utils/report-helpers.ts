/**
 * 周报聚合纯函数：日期区间计算、提交解析与 Markdown 草稿生成。
 *
 * 全部为无副作用函数，便于单元测试；调用方（视图层）负责拉取数据。
 */

import type { RepoCommitLog, WeeklyReportCommit } from '@/types/weekly-report';

/** 统计区间（Unix 毫秒，含两端） */
export interface DateRange {
  start: number;
  end: number;
}

export type RangePreset = 'this_week' | 'last_week' | 'last_7_days' | 'last_30_days';

/** 草稿分组方式：按提交类型或按仓库 */
export type ReportGroupBy = 'type' | 'repo';

/** 常见 conventional commit 类型与中文标题，顺序即草稿小节顺序 */
export const COMMIT_TYPE_LABELS: Readonly<Record<string, string>> = {
  feat: '新功能',
  fix: '缺陷修复',
  perf: '性能优化',
  refactor: '重构',
  docs: '文档',
  test: '测试',
  style: '样式调整',
  build: '构建',
  ci: '持续集成',
  chore: '其他',
};

const TYPE_ORDER = Object.keys(COMMIT_TYPE_LABELS);

const DAY_MS = 24 * 60 * 60 * 1000;

// ===== 日期区间 =====

/** 本周（周一 00:00:00.000 至周日 23:59:59.999，本地时区） */
export function getThisWeekRange(now = new Date()): DateRange {
  const start = new Date(now);
  const daysSinceMonday = (start.getDay() + 6) % 7;
  start.setDate(start.getDate() - daysSinceMonday);
  start.setHours(0, 0, 0, 0);

  const end = new Date(start);
  end.setDate(end.getDate() + 6);
  end.setHours(23, 59, 59, 999);

  return { start: start.getTime(), end: end.getTime() };
}

/** 上周（同样以周一为起点） */
export function getLastWeekRange(now = new Date()): DateRange {
  const thisWeek = getThisWeekRange(now);
  const week = 7 * DAY_MS;
  return { start: thisWeek.start - week, end: thisWeek.end - week };
}

/** 最近 N 天（含今天） */
export function getDaysRange(days: number, now = new Date()): DateRange {
  const start = new Date(now);
  start.setDate(start.getDate() - (days - 1));
  start.setHours(0, 0, 0, 0);

  const end = new Date(now);
  end.setHours(23, 59, 59, 999);

  return { start: start.getTime(), end: end.getTime() };
}

export function getPresetRange(preset: RangePreset, now = new Date()): DateRange {
  switch (preset) {
    case 'this_week':
      return getThisWeekRange(now);
    case 'last_week':
      return getLastWeekRange(now);
    case 'last_7_days':
      return getDaysRange(7, now);
    case 'last_30_days':
      return getDaysRange(30, now);
  }
}

// ===== 格式化 =====

/** `2026.09.28` 形式的日期 */
export function formatDate(ms: number): string {
  const date = new Date(ms);
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}.${month}.${day}`;
}

/** `2026.09.28 - 2026.10.04` 形式的区间 */
export function formatRange(range: DateRange): string {
  return `${formatDate(range.start)} - ${formatDate(range.end)}`;
}

/** `2026-09-28 14:30` 形式的日期时间 */
export function formatDateTime(ms: number): string {
  const date = new Date(ms);
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hour = String(date.getHours()).padStart(2, '0');
  const minute = String(date.getMinutes()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day} ${hour}:${minute}`;
}

/** 周报默认标题 */
export function buildReportTitle(range: DateRange): string {
  return `周报 ${formatRange(range)}`;
}

// ===== 提交解析 =====

export interface ParsedCommitSubject {
  /** conventional 类型（小写），非规范提交为空串 */
  type: string;
  /** conventional scope，可为空串 */
  scope: string;
  /** 去掉类型前缀后的正文 */
  text: string;
}

/**
 * 解析 conventional commit 主题：`feat(mail): 支持模板` → feat / mail / 支持模板。
 * 不符合规范的提交原样返回（type 为空串）。
 */
export function parseCommitSubject(subject: string): ParsedCommitSubject {
  const trimmed = subject.trim();
  const match = /^([A-Z]+)(?:\(([^)]*)\))?!?\s*:\s*(.+)$/i.exec(trimmed);

  if (!match) {
    return { type: '', scope: '', text: trimmed };
  }

  return {
    type: match[1] ? match[1].toLowerCase() : '',
    scope: match[2]?.trim() ?? '',
    text: match[3] ? match[3].trim() : trimmed,
  };
}

// ===== 聚合 =====

export interface CollectedCommits {
  /** 按时间升序、跨仓库去重（hash）后的提交，附带所属仓库名 */
  commits: Array<WeeklyReportCommit & { repo_name: string }>;
  /** 拉取失败的仓库与原因 */
  errors: Array<{ repo_name: string; error: string }>;
  /** 成功拉取的仓库数 */
  repo_count: number;
}

/** 展平各仓库提交：全局按 hash 去重，失败仓库归入 errors */
export function collectCommits(logs: RepoCommitLog[]): CollectedCommits {
  const seen = new Set<string>();
  const commits: CollectedCommits['commits'] = [];
  const errors: CollectedCommits['errors'] = [];
  let repoCount = 0;

  for (const log of logs) {
    if (log.error) {
      errors.push({ repo_name: log.repo_name, error: log.error });
      continue;
    }

    repoCount += 1;
    for (const commit of log.commits) {
      if (seen.has(commit.hash)) {
        continue;
      }
      seen.add(commit.hash);
      commits.push({ ...commit, repo_name: log.repo_name });
    }
  }

  commits.sort((a, b) => a.timestamp - b.timestamp);
  return { commits, errors, repo_count: repoCount };
}

/** 单条提交的展示文本（不含列表符号） */
function formatCommitLine(
  commit: CollectedCommits['commits'][number],
  groupBy: ReportGroupBy,
): string {
  const parsed = parseCommitSubject(commit.subject);

  if (groupBy === 'repo') {
    const prefix = parsed.type ? `${parsed.type}${parsed.scope ? `(${parsed.scope})` : ''}: ` : '';
    return `${prefix}${parsed.text}`;
  }

  const context = parsed.scope ? `${parsed.scope} · ${commit.repo_name}` : commit.repo_name;
  return `${parsed.text}（${context}）`;
}

/**
 * 把提交聚合成周报 Markdown 草稿。
 *
 * 结构：标题 + 统计行 + 分组小节 + 失败仓库备注。
 * 没有任何提交时返回空串，由调用方决定空态展示。
 */
export function aggregateCommits(
  logs: RepoCommitLog[],
  options: { range: DateRange; groupBy: ReportGroupBy },
): string {
  const { commits, errors, repo_count: repoCount } = collectCommits(logs);

  if (commits.length === 0 && errors.length === 0) {
    return '';
  }

  const lines: string[] = [
    `# ${buildReportTitle(options.range)}`,
    '',
    `> 统计区间：${formatRange(options.range)} ｜ ${repoCount} 个仓库 ｜ ${commits.length} 次提交`,
  ];

  if (commits.length > 0) {
    lines.push('', ...(options.groupBy === 'type' ? buildTypeSections(commits) : buildRepoSections(commits)));
  }

  if (errors.length > 0) {
    lines.push('', '> ⚠️ 以下仓库拉取失败，未计入统计：');
    for (const item of errors) {
      lines.push(`> - ${item.repo_name}：${item.error}`);
    }
  }

  return lines.join('\n');
}

/** 按提交类型分组；非规范提交归入「其他」，与 chore 同节 */
function buildTypeSections(commits: CollectedCommits['commits']): string[] {
  const groups = new Map<string, CollectedCommits['commits']>();

  for (const commit of commits) {
    const parsed = parseCommitSubject(commit.subject);
    const key = COMMIT_TYPE_LABELS[parsed.type] ? parsed.type : 'chore';
    const bucket = groups.get(key);
    if (bucket) {
      bucket.push(commit);
    } else {
      groups.set(key, [commit]);
    }
  }

  const orderedTypes = [
    ...TYPE_ORDER.filter(type => groups.has(type)),
    ...[...groups.keys()].filter(type => !TYPE_ORDER.includes(type)),
  ];

  const lines: string[] = [];
  for (const type of orderedTypes) {
    const bucket = groups.get(type);
    if (!bucket) {
      continue;
    }
    lines.push('', `## ${COMMIT_TYPE_LABELS[type] ?? type}`, '');
    for (const commit of bucket) {
      lines.push(`- ${formatCommitLine(commit, 'type')}`);
    }
  }
  return lines;
}

/** 按仓库分组 */
function buildRepoSections(commits: CollectedCommits['commits']): string[] {
  const groups = new Map<string, CollectedCommits['commits']>();

  for (const commit of commits) {
    const bucket = groups.get(commit.repo_name);
    if (bucket) {
      bucket.push(commit);
    } else {
      groups.set(commit.repo_name, [commit]);
    }
  }

  const lines: string[] = [];
  for (const [repoName, bucket] of groups) {
    lines.push('', `## ${repoName}`, '');
    for (const commit of bucket) {
      lines.push(`- ${formatCommitLine(commit, 'repo')}`);
    }
  }
  return lines;
}

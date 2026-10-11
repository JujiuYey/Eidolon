/**
 * 周报页统计（纯函数，可单测）。
 *
 * 仓库数、周报数（累计/本月）、累计提交量、最近保存时间。
 */

import type { WeeklyReportEntry, WeeklyReportRepo } from '@/types/weekly-report';
import { countSince, formatShortDate, startOfDay, DAY_MS } from '@/utils/stats';

/** 周报统计瓦片 */
export interface WeeklyReportStats {
  repoCount: number;
  reportCount: number;
  totalCommitCount: number;
  monthReportCount: number;
  /** 最近一份周报的保存时间，无记录为 0 */
  lastReportAt: number;
}

export function computeWeeklyReportStats(
  reports: WeeklyReportEntry[],
  repos: WeeklyReportRepo[],
  now = Date.now(),
): WeeklyReportStats {
  const reportTimestamps = reports.map(report => report.created_at);

  return {
    repoCount: repos.length,
    reportCount: reports.length,
    totalCommitCount: reports.reduce((sum, report) => sum + report.commit_count, 0),
    monthReportCount: countSince(reportTimestamps, startOfDay(now) - 29 * DAY_MS),
    lastReportAt: reports.reduce((latest, report) => Math.max(latest, report.created_at), 0),
  };
}

/** 「累计周报」瓦片的提示文案：最近保存时间 */
export function lastReportHint(lastReportAt: number): string {
  return lastReportAt ? `最近 ${formatShortDate(lastReportAt)}` : '';
}

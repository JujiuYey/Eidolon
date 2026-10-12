/**
 * 文档管理页统计（纯函数，可单测）。
 *
 * 库数量 + 各库条目快照（list_docs_entries）汇总出的文档总数、
 * 30 天内更新数与更新分布；目录条目不计数。
 */

import type { DocsEntry } from '@/types/docs';
import { countSince, dailyCounts, dailyTooltips, formatShortDate, startOfDay, DAY_MS } from '@/utils/stats';

/** 文档库统计瓦片 + 更新趋势 */
export interface DocsStats {
  rootCount: number;
  docCount: number;
  recentUpdatedCount: number;
  /** 所有文档里最新的 updated_at，无文档为 0 */
  lastUpdatedAt: number;
  dailyUpdated: number[];
  dailyTooltips: string[];
}

/** entries 按库传入（每库一份 list_docs_entries 快照） */
export function computeDocsStats(rootCount: number, entries: DocsEntry[][], now = Date.now()): DocsStats {
  const files = entries.flat().filter(entry => !entry.is_dir);
  const timestamps = files.map(entry => entry.updated_at);
  const daily = dailyCounts(timestamps, 30, now);

  return {
    rootCount,
    docCount: files.length,
    recentUpdatedCount: countSince(timestamps, startOfDay(now) - 29 * DAY_MS),
    lastUpdatedAt: timestamps.reduce((latest, timestamp) => Math.max(latest, timestamp), 0),
    dailyUpdated: daily,
    dailyTooltips: dailyTooltips(daily, '篇更新', now),
  };
}

/** 「文档总数」瓦片的提示文案：最近更新时间 */
export function lastUpdatedHint(lastUpdatedAt: number): string {
  return lastUpdatedAt ? `最近 ${formatShortDate(lastUpdatedAt)}` : '';
}

/** 每库文档数条形榜（labels 与 entries 按库一一对应），仅统计文件 */
export function rootDocStats(labels: string[], entries: DocsEntry[][]): Array<{ label: string; value: number }> {
  return labels.map((label, index) => ({
    label,
    value: (entries[index] ?? []).filter(entry => !entry.is_dir).length,
  }));
}

/**
 * 收藏夹展示辅助函数（纯函数，可单测）。
 *
 * 链接/记事类型判断、域名提取、列表过滤与计数、时间格式化。
 * 后端 list 已按 created_at 倒序返回，前端不再排序。
 */

import type { CollectItem, CollectKind } from '@/types/collect';
import {
  consecutiveDays,
  countSince,
  dailyCounts,
  dailyTooltips,
  DAY_MS,
  startOfDay,
  weekStartMs,
} from '@/utils/stats';

/** http(s) 开头的非空白文本按链接处理，其余按记事 */
export function isLinkText(text: string): boolean {
  return /^https?:\/\/\S+$/i.test(text.trim());
}

/** 从 URL 提取域名作占位标题；解析失败时原样返回 */
export function hostOf(url: string | null): string {
  if (!url) {
    return '';
  }
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

export type CollectFilter = 'all' | CollectKind | 'archived';

/** 各过滤项的条目数（archived 与 all/link/note 互斥统计） */
export function filterCounts(items: CollectItem[]): Record<CollectFilter, number> {
  const active = items.filter(item => !item.archived);
  return {
    all: active.length,
    link: active.filter(item => item.kind === 'link').length,
    note: active.filter(item => item.kind === 'note').length,
    archived: items.length - active.length,
  };
}

/** 按过滤项取条目；全部/链接/记事不含归档，归档只在归档页出现。
 *  query 非空时在标题/摘要/备注正文/URL 里做包含匹配 */
export function filterItems(items: CollectItem[], filter: CollectFilter, query: string): CollectItem[] {
  let result = items;
  if (filter === 'archived') {
    result = result.filter(item => item.archived);
  } else {
    result = result.filter(item => !item.archived);
    if (filter !== 'all') {
      result = result.filter(item => item.kind === filter);
    }
  }

  const keyword = query.trim().toLowerCase();
  if (!keyword) {
    return result;
  }
  return result.filter(item => searchableText(item).includes(keyword));
}

function searchableText(item: CollectItem): string {
  return [item.title ?? '', item.description ?? '', item.content, item.url ?? ''].join('\n').toLowerCase();
}

/** 列表展示时间：Unix 毫秒 → 'YYYY-MM-DD'（本地时区） */
export function formatCreatedAt(timestamp: number): string {
  if (!timestamp) {
    return '';
  }
  const date = new Date(timestamp);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** 统计用的域名提取：只认 http(s) 的合法 URL，其余（含 hostOf 的原样兜底）不计数 */
function hostForStats(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:' ? parsed.host : '';
  } catch {
    return '';
  }
}

/** 收藏夹统计瓦片 + 趋势 + 常收藏站点 */ export interface CollectStats {
  weekCount: number;
  monthCount: number;
  streakDays: number;
  archivedCount: number;
  dailyCounts: number[];
  dailyTooltips: string[];
  topHosts: Array<{ host: string; count: number }>;
}

/** 从全量条目（含归档）计算统计：数量、连续天数、30 天分布、Top 域名 */
export function computeCollectStats(items: CollectItem[], now = Date.now()): CollectStats {
  const timestamps = items.map(item => item.created_at);
  const daily = dailyCounts(timestamps, 30, now);

  const hostCounts = new Map<string, number>();
  for (const item of items) {
    if (item.kind !== 'link' || !item.url) {
      continue;
    }
    const host = hostForStats(item.url);
    if (!host) {
      continue;
    }
    hostCounts.set(host, (hostCounts.get(host) ?? 0) + 1);
  }

  return {
    weekCount: countSince(timestamps, weekStartMs(now)),
    monthCount: countSince(timestamps, startOfDay(now) - 29 * DAY_MS),
    streakDays: consecutiveDays(timestamps, now),
    archivedCount: items.filter(item => item.archived).length,
    dailyCounts: daily,
    dailyTooltips: dailyTooltips(daily, '条收藏', now),
    topHosts: [...hostCounts.entries()]
      .map(([host, count]) => ({ host, count }))
      .sort((a, b) => b.count - a.count || a.host.localeCompare(b.host))
      .slice(0, 5),
  };
}

/** 回收站保留期（天），与后端 RETENTION_MS 对齐 */
export const TRASH_RETENTION_DAYS = 30;

/** 回收站条目的剩余保留天数（按自然日粗算，最少 0） */
export function remainingDays(deletedAt: number | null, now = Date.now()): number {
  if (!deletedAt) {
    return TRASH_RETENTION_DAYS;
  }
  const elapsedDays = Math.floor((now - deletedAt) / DAY_MS);
  return Math.max(0, TRASH_RETENTION_DAYS - elapsedDays);
}

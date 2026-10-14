/**
 * 收藏夹展示辅助函数（纯函数，可单测）。
 *
 * 链接/记事类型判断、域名提取、列表过滤与计数、时间格式化。
 * 后端 list 已按 created_at 倒序返回，前端不再排序。
 */

import type { CollectItem, CollectKind } from '@/types/collect';
import { DAY_MS } from '@/utils/stats';

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

/** 来源筛选用的域名提取：只认 http(s) 的合法 URL，小写并去掉前导 www.；其余返回空串 */
export function sourceOf(url: string | null): string {
  if (!url) {
    return '';
  }
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      return '';
    }
    return parsed.host.replace(/^www\./i, '').toLowerCase();
  } catch {
    return '';
  }
}

/** 来源下拉的选项（仅未归档链接），按条数倒序、同数按域名排序 */
export function sourceOptions(items: CollectItem[]): Array<{ host: string; count: number }> {
  const counts = new Map<string, number>();
  for (const item of items) {
    if (item.archived || item.kind !== 'link') {
      continue;
    }
    const host = sourceOf(item.url);
    if (!host) {
      continue;
    }
    counts.set(host, (counts.get(host) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([host, count]) => ({ host, count }))
    .sort((a, b) => b.count - a.count || a.host.localeCompare(b.host));
}

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
 *  source 非空时再按来源（归一化域名）精确过滤，只对链接生效；
 *  query 非空时在标题/摘要/备注正文/URL 里做包含匹配 */
export function filterItems(items: CollectItem[], filter: CollectFilter, query: string, source = ''): CollectItem[] {
  let result = items;
  if (filter === 'archived') {
    result = result.filter(item => item.archived);
  } else {
    result = result.filter(item => !item.archived);
    if (filter !== 'all') {
      result = result.filter(item => item.kind === filter);
    }
  }
  if (source) {
    result = result.filter(item => item.kind === 'link' && sourceOf(item.url) === source);
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

/** 常收藏站点：按链接条目数降序取前 N，同数按域名排序；只统计可解析出域名的 http(s) 链接 */
export function topCollectHosts(items: CollectItem[], limit = 5): Array<{ host: string; count: number }> {
  const hostCounts = new Map<string, number>();
  for (const item of items) {
    if (item.kind !== 'link' || !item.url) {
      continue;
    }
    const host = sourceOf(item.url);
    if (!host) {
      continue;
    }
    hostCounts.set(host, (hostCounts.get(host) ?? 0) + 1);
  }

  return [...hostCounts.entries()]
    .map(([host, count]) => ({ host, count }))
    .sort((a, b) => b.count - a.count || a.host.localeCompare(b.host))
    .slice(0, limit);
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

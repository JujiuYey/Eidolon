/**
 * 收藏夹展示辅助函数（纯函数，可单测）。
 *
 * 链接/记事类型判断、域名提取、列表过滤与计数、时间格式化。
 * 后端 list 已按 created_at 倒序返回，前端不再排序。
 */

import type { CollectItem, CollectKind } from '@/types/collect';

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

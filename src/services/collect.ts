import { invoke } from '@tauri-apps/api/core';

import type { CollectItem, LinkMetadata } from '@/types/collect';

export async function listCollectItems(): Promise<CollectItem[]> {
  return invoke<CollectItem[]>('list_collect_items');
}

export async function upsertCollectItem(item: CollectItem): Promise<CollectItem> {
  return invoke<CollectItem>('upsert_collect_item', { item });
}

export async function deleteCollectItem(itemId: string): Promise<string> {
  return invoke<string>('delete_collect_item', { itemId });
}

/** 抓取链接的标题/摘要/favicon；保存链接后异步调用回填 */
export async function fetchLinkMetadata(url: string): Promise<LinkMetadata> {
  return invoke<LinkMetadata>('fetch_link_metadata', { url });
}

/** AI：总结单条链接（现场抓正文，不落库） */
export async function summarizeCollectLink(url: string): Promise<string> {
  return invoke<string>('summarize_collect_link', { url });
}

/** AI：基于全部收藏条目问答 */
export async function askCollectItems(question: string): Promise<string> {
  return invoke<string>('ask_collect_items', { question });
}

/** 在系统默认浏览器中打开链接（后端只放行 http/https） */
export async function openInBrowser(url: string): Promise<void> {
  return invoke<void>('open_external_url', { rawUrl: url });
}

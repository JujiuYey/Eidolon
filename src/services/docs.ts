import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type { DocsEntry, DocsExternalChange, DocsSearchResult, DocsSettings } from '@/types/docs';

export async function getDocsSettings(): Promise<DocsSettings> {
  return invoke<DocsSettings>('get_docs_settings');
}

export async function upsertDocsSettings(settings: DocsSettings): Promise<DocsSettings> {
  return invoke<DocsSettings>('upsert_docs_settings', { settings });
}

export async function listDocsEntries(rootId: string): Promise<DocsEntry[]> {
  return invoke<DocsEntry[]>('list_docs_entries', { rootId });
}

export async function readDocsFile(rootId: string, path: string): Promise<string> {
  return invoke<string>('read_docs_file', { rootId, path });
}

export async function saveDocsFile(rootId: string, path: string, content: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('save_docs_file', { rootId, path, content });
}

export async function createDocsFile(rootId: string, dirPath: string, name: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('create_docs_file', { rootId, dirPath, name });
}

export async function createDocsDirectory(rootId: string, parentPath: string, name: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('create_docs_directory', { rootId, parentPath, name });
}

export async function renameDocsEntry(rootId: string, path: string, newName: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('rename_docs_entry', { rootId, path, newName });
}

export async function deleteDocsEntry(rootId: string, path: string): Promise<string> {
  return invoke<string>('delete_docs_entry', { rootId, path });
}

export async function searchDocs(rootId: string, keyword: string): Promise<DocsSearchResult[]> {
  return invoke<DocsSearchResult[]>('search_docs', { rootId, keyword });
}

/** 后端广播的文档外部变更事件（MCP agent 写入文档库时发出） */
export const DOCS_EXTERNAL_CHANGE_EVENT = 'docs-external-change';

/** 监听文档外部变更，返回取消监听的函数（工作区卸载时调用） */
export function onDocsExternalChange(handler: (change: DocsExternalChange) => void): Promise<UnlistenFn> {
  return listen<DocsExternalChange>(DOCS_EXTERNAL_CHANGE_EVENT, event => handler(event.payload));
}

export interface DocsAiChatMessage {
  role: 'user' | 'assistant';
  content: string;
}

export interface DocsAiChatResult {
  content: string;
  model_label: string;
}

/** 文档 AI 助手一轮对话。会话历史由前端持有，后端无状态 */
export async function chatDocsAi(
  history: DocsAiChatMessage[],
  document: string,
  message: string,
): Promise<DocsAiChatResult> {
  return invoke<DocsAiChatResult>('chat_docs_ai', { history, document, message });
}

import type { DocsEntry, DocsSearchResult, DocsSettings } from '@/types/docs';
import { invoke } from '@tauri-apps/api/core';

export async function getDocsSettings(): Promise<DocsSettings> {
  return invoke<DocsSettings>('get_docs_settings');
}

export async function upsertDocsSettings(settings: DocsSettings): Promise<DocsSettings> {
  return invoke<DocsSettings>('upsert_docs_settings', { settings });
}

export async function listDocsEntries(): Promise<DocsEntry[]> {
  return invoke<DocsEntry[]>('list_docs_entries');
}

export async function readDocsFile(path: string): Promise<string> {
  return invoke<string>('read_docs_file', { path });
}

export async function saveDocsFile(path: string, content: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('save_docs_file', { path, content });
}

export async function createDocsFile(dirPath: string, name: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('create_docs_file', { dirPath, name });
}

export async function createDocsDirectory(parentPath: string, name: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('create_docs_directory', { parentPath, name });
}

export async function renameDocsEntry(path: string, newName: string): Promise<DocsEntry> {
  return invoke<DocsEntry>('rename_docs_entry', { path, newName });
}

export async function deleteDocsEntry(path: string): Promise<string> {
  return invoke<string>('delete_docs_entry', { path });
}

export async function searchDocs(keyword: string): Promise<DocsSearchResult[]> {
  return invoke<DocsSearchResult[]>('search_docs', { keyword });
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

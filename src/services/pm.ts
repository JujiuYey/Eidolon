import type { PmConversation, PmMessage, PmSettings, PmSkillSummary } from '@/types/pm';
import { invoke } from '@tauri-apps/api/core';

export async function listPmConversations(): Promise<PmConversation[]> {
  return invoke<PmConversation[]>('list_pm_conversations');
}

export async function createPmConversation(): Promise<PmConversation> {
  return invoke<PmConversation>('create_pm_conversation');
}

export async function renamePmConversation(
  conversationId: string,
  title: string,
): Promise<PmConversation> {
  return invoke<PmConversation>('rename_pm_conversation', { conversationId, title });
}

export async function deletePmConversation(conversationId: string): Promise<string> {
  return invoke<string>('delete_pm_conversation', { conversationId });
}

export async function listPmConversationMessages(conversationId: string): Promise<PmMessage[]> {
  return invoke<PmMessage[]>('list_pm_conversation_messages', { conversationId });
}

export async function sendPmMessage(conversationId: string, content: string): Promise<PmMessage> {
  return invoke<PmMessage>('send_pm_message', { conversationId, content });
}

export async function listPmSkills(): Promise<PmSkillSummary[]> {
  return invoke<PmSkillSummary[]>('list_pm_skills');
}

export async function getPmSkillContent(slug: string): Promise<string> {
  return invoke<string>('get_pm_skill_content', { slug });
}

export async function getPmSettings(): Promise<PmSettings> {
  return invoke<PmSettings>('get_pm_settings');
}

export async function upsertPmSettings(settings: PmSettings): Promise<PmSettings> {
  return invoke<PmSettings>('upsert_pm_settings', { settings });
}

/** 在系统文件管理器中打开目录（Skills 覆盖目录用） */
export async function revealDirectory(path: string): Promise<void> {
  await invoke('open_directory', { path });
}

import type { AgentMessage } from '@/types';
const AGENT_CONVERSATION_STORAGE_KEY = 'eidolon.agent_conversations';

type AgentConversationMap = Record<string, AgentMessage[]>;

function canUseStorage() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readStorage<T>(key: string, fallback: T): T {
  if (!canUseStorage()) {
    return fallback;
  }

  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) {
      return fallback;
    }

    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeStorage<T>(key: string, value: T) {
  if (!canUseStorage()) {
    return;
  }

  window.localStorage.setItem(key, JSON.stringify(value));
}

export function listAgentConversationMessages(profileId: string): AgentMessage[] {
  const conversations = readStorage<AgentConversationMap>(AGENT_CONVERSATION_STORAGE_KEY, {});
  return conversations[profileId] ?? [];
}

export function saveAgentConversationMessages(profileId: string, messages: AgentMessage[]) {
  const conversations = readStorage<AgentConversationMap>(AGENT_CONVERSATION_STORAGE_KEY, {});
  conversations[profileId] = messages;
  writeStorage(AGENT_CONVERSATION_STORAGE_KEY, conversations);
}

import { nanoid } from 'nanoid';
import type {
  AgentMessage,
  AgentProfile,
  AgentProfileInput,
} from '@/types';

const AGENT_PROFILE_STORAGE_KEY = 'eidolon.agent_profiles';
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

export function listAgentProfiles(): AgentProfile[] {
  const profiles = readStorage<AgentProfile[]>(AGENT_PROFILE_STORAGE_KEY, []);

  return [...profiles].sort((left, right) => right.updatedAt - left.updatedAt);
}

export function getAgentProfile(profileId: string): AgentProfile | null {
  return listAgentProfiles().find(profile => profile.id === profileId) ?? null;
}

export function upsertAgentProfile(input: AgentProfileInput): AgentProfile {
  const profiles = listAgentProfiles();
  const now = Date.now();
  const existing = input.id ? profiles.find(profile => profile.id === input.id) ?? null : null;

  const nextProfile: AgentProfile = {
    id: existing?.id ?? input.id ?? `agent-${nanoid(10)}`,
    name: input.name.trim(),
    description: input.description.trim(),
    providerId: input.providerId,
    modelId: input.modelId,
    temperature: input.temperature.trim(),
    maxTokens: input.maxTokens.trim(),
    systemPrompt: input.systemPrompt.trim(),
    enabledMcpServiceIds: [...input.enabledMcpServiceIds],
    enabledToolKeys: [...input.enabledToolKeys],
    createdAt: existing?.createdAt ?? now,
    updatedAt: now,
  };

  const nextProfiles = existing
    ? profiles.map(profile => (profile.id === nextProfile.id ? nextProfile : profile))
    : [nextProfile, ...profiles];

  writeStorage(AGENT_PROFILE_STORAGE_KEY, nextProfiles);
  return nextProfile;
}

export function deleteAgentProfile(profileId: string) {
  const profiles = listAgentProfiles().filter(profile => profile.id !== profileId);
  const conversations = readStorage<AgentConversationMap>(AGENT_CONVERSATION_STORAGE_KEY, {});
  const { [profileId]: _removedConversation, ...restConversations } = conversations;

  writeStorage(AGENT_PROFILE_STORAGE_KEY, profiles);
  writeStorage(AGENT_CONVERSATION_STORAGE_KEY, restConversations);
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

import { invoke } from '@tauri-apps/api/core';
import type {
  AgentProfile,
  AgentProfileInput,
} from '@/types';

const LEGACY_AGENT_PROFILE_STORAGE_KEY = 'eidolon.agent_profiles';
const AGENT_PROFILE_MIGRATION_KEY = 'eidolon.agent_profiles.migrated_to_tauri';

interface TauriAgentProfile {
  id: string;
  name: string;
  description: string;
  provider_id: string;
  model_id: string;
  temperature: string;
  max_tokens: string;
  system_prompt: string;
  enabled_mcp_service_ids: string[];
  enabled_tool_keys: string[];
  created_at: number;
  updated_at: number;
}

let migrationPromise: Promise<void> | null = null;

function canUseStorage() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readLegacyProfiles(): AgentProfile[] {
  if (!canUseStorage()) {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(LEGACY_AGENT_PROFILE_STORAGE_KEY);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed as AgentProfile[] : [];
  } catch {
    return [];
  }
}

function markMigrationDone() {
  if (!canUseStorage()) {
    return;
  }

  window.localStorage.setItem(AGENT_PROFILE_MIGRATION_KEY, 'done');
}

function isMigrationDone() {
  return canUseStorage() && window.localStorage.getItem(AGENT_PROFILE_MIGRATION_KEY) === 'done';
}

function toFrontendProfile(profile: TauriAgentProfile): AgentProfile {
  return {
    id: profile.id,
    name: profile.name,
    description: profile.description,
    providerId: profile.provider_id,
    modelId: profile.model_id,
    temperature: profile.temperature,
    maxTokens: profile.max_tokens,
    systemPrompt: profile.system_prompt,
    enabledMcpServiceIds: profile.enabled_mcp_service_ids,
    enabledToolKeys: profile.enabled_tool_keys,
    createdAt: profile.created_at,
    updatedAt: profile.updated_at,
  };
}

function toTauriProfile(
  profile: AgentProfileInput | AgentProfile,
  fallbackId?: string,
): TauriAgentProfile {
  return {
    id: ('id' in profile ? profile.id : fallbackId) ?? '',
    name: profile.name,
    description: profile.description,
    provider_id: profile.providerId,
    model_id: profile.modelId,
    temperature: profile.temperature,
    max_tokens: profile.maxTokens,
    system_prompt: profile.systemPrompt,
    enabled_mcp_service_ids: [...profile.enabledMcpServiceIds],
    enabled_tool_keys: [...profile.enabledToolKeys],
    created_at: 'createdAt' in profile ? profile.createdAt : 0,
    updated_at: 'updatedAt' in profile ? profile.updatedAt : 0,
  };
}

async function invokeListAgentProfiles() {
  const profiles = await invoke<TauriAgentProfile[]>('list_agent_profiles');
  return profiles.map(toFrontendProfile);
}

async function ensureAgentProfileMigration() {
  if (migrationPromise) {
    return migrationPromise;
  }

  migrationPromise = (async () => {
    if (isMigrationDone()) {
      return;
    }

    const tauriProfiles = await invokeListAgentProfiles();
    if (tauriProfiles.length > 0) {
      markMigrationDone();
      return;
    }

    const legacyProfiles = readLegacyProfiles();
    for (const profile of legacyProfiles) {
      const tauriProfile = toTauriProfile(profile);

      if (!tauriProfile.name.trim()
        || !tauriProfile.provider_id.trim()
        || !tauriProfile.model_id.trim()
        || !tauriProfile.system_prompt.trim()) {
        continue;
      }

      await invoke<string>('upsert_agent_profile', { profile: tauriProfile });
    }

    markMigrationDone();
  })().finally(() => {
    migrationPromise = null;
  });

  return migrationPromise;
}

export async function listAgentProfiles(): Promise<AgentProfile[]> {
  await ensureAgentProfileMigration();
  return invokeListAgentProfiles();
}

export async function getAgentProfile(profileId: string): Promise<AgentProfile | null> {
  await ensureAgentProfileMigration();
  const profile = await invoke<TauriAgentProfile | null>('get_agent_profile', { profileId });
  return profile ? toFrontendProfile(profile) : null;
}

export async function upsertAgentProfile(input: AgentProfileInput): Promise<AgentProfile> {
  await ensureAgentProfileMigration();
  const profile = toTauriProfile(input, input.id);
  const profileId = await invoke<string>('upsert_agent_profile', { profile });
  const persisted = await getAgentProfile(profileId);

  if (!persisted) {
    throw new Error('Agent 保存后读取失败');
  }

  return persisted;
}

export async function deleteAgentProfile(profileId: string): Promise<string> {
  await ensureAgentProfileMigration();
  return invoke<string>('delete_agent_profile', { profileId });
}

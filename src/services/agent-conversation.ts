import { invoke } from '@tauri-apps/api/core';
import type { AgentWorkspaceConversation, AgentWorkspaceMessage } from '@/types/agent';

// Snake case response from Tauri
interface TauriAgentConversation {
  id: string;
  agent_profile_id: string;
  title: string;
  snapshot_version: number;
  created_from_profile_updated_at: number;
  snapshot_agent_name: string;
  snapshot_provider_id: string;
  snapshot_model_id: string;
  snapshot_temperature: string;
  snapshot_max_tokens: string;
  snapshot_system_prompt: string;
  snapshot_enabled_mcp_service_ids: string[];
  snapshot_enabled_tool_keys: string[];
  created_at: number;
  updated_at: number;
}

interface TauriAgentConversationMessage {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  status: string;
  created_at: number;
}

// Convert snake_case to camelCase
function toCamelCaseConversation(raw: TauriAgentConversation): AgentWorkspaceConversation {
  return {
    id: raw.id,
    agentProfileId: raw.agent_profile_id,
    title: raw.title,
    snapshotVersion: raw.snapshot_version,
    createdFromProfileUpdatedAt: raw.created_from_profile_updated_at,
    snapshotAgentName: raw.snapshot_agent_name,
    snapshotProviderId: raw.snapshot_provider_id,
    snapshotModelId: raw.snapshot_model_id,
    snapshotTemperature: raw.snapshot_temperature,
    snapshotMaxTokens: raw.snapshot_max_tokens,
    snapshotSystemPrompt: raw.snapshot_system_prompt,
    snapshotEnabledMcpServiceIds: raw.snapshot_enabled_mcp_service_ids,
    snapshotEnabledToolKeys: raw.snapshot_enabled_tool_keys,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function toCamelCaseMessage(raw: TauriAgentConversationMessage): AgentWorkspaceMessage {
  return {
    id: raw.id,
    conversationId: raw.conversation_id,
    role: raw.role as 'user' | 'assistant' | 'system',
    content: raw.content,
    status: raw.status as 'done' | 'error',
    createdAt: raw.created_at,
  };
}

/**
 * List all conversations for a specific agent profile
 */
export async function listAgentConversations(
  agentProfileId: string,
): Promise<AgentWorkspaceConversation[]> {
  const raw = await invoke<TauriAgentConversation[]>('list_agent_conversations', {
    agentProfileId,
  });
  return raw.map(toCamelCaseConversation);
}

/**
 * List recent conversations across all agents
 */
export async function listRecentAgentConversations(
  limit?: number,
): Promise<AgentWorkspaceConversation[]> {
  const raw = await invoke<TauriAgentConversation[]>('list_recent_agent_conversations', {
    limit,
  });
  return raw.map(toCamelCaseConversation);
}

/**
 * Create a new conversation from an agent profile
 */
export async function createAgentConversation(
  agentProfileId: string,
): Promise<AgentWorkspaceConversation> {
  const raw = await invoke<TauriAgentConversation>('create_agent_conversation', {
    agentProfileId,
  });
  return toCamelCaseConversation(raw);
}

/**
 * Get a single conversation by ID
 */
export async function getAgentConversation(
  conversationId: string,
): Promise<AgentWorkspaceConversation | null> {
  const raw = await invoke<TauriAgentConversation | null>('get_agent_conversation', {
    conversationId,
  });
  return raw ? toCamelCaseConversation(raw) : null;
}

/**
 * Delete a conversation
 */
export async function deleteAgentConversation(conversationId: string): Promise<string> {
  return invoke<string>('delete_agent_conversation', { conversationId });
}

/**
 * List all messages in a conversation
 */
export async function listAgentConversationMessages(
  conversationId: string,
): Promise<AgentWorkspaceMessage[]> {
  const raw = await invoke<TauriAgentConversationMessage[]>(
    'list_agent_conversation_messages',
    { conversationId },
  );
  return raw.map(toCamelCaseMessage);
}

/**
 * Send a message and get the agent's response
 */
export async function sendAgentConversationMessage(
  conversationId: string,
  content: string,
): Promise<AgentWorkspaceMessage> {
  const raw = await invoke<TauriAgentConversationMessage>(
    'send_agent_conversation_message',
    { conversationId, content },
  );
  return toCamelCaseMessage(raw);
}

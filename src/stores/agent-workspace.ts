import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import {
  createAgentConversation,
  deleteAgentConversation,
  getAgentConversation,
  listAgentConversationMessages,
  listRecentAgentConversations,
  sendAgentConversationMessage,
} from '@/services/agent-conversation';
import type { AgentWorkspaceConversation, AgentWorkspaceMessage } from '@/types';

const DEFAULT_RECENT_CONVERSATION_LIMIT = 12;

export const useAgentWorkspaceStore = defineStore('agent-workspace', () => {
  const recentConversations = ref<AgentWorkspaceConversation[]>([]);
  const activeConversation = ref<AgentWorkspaceConversation | null>(null);
  const messages = ref<AgentWorkspaceMessage[]>([]);
  const isLoadingRecent = ref(false);
  const isLoadingConversation = ref(false);
  const isCreatingConversation = ref(false);
  const isSending = ref(false);
  const deletingConversationId = ref<string | null>(null);
  let conversationRequestId = 0;

  const activeConversationId = computed(() => activeConversation.value?.id ?? null);

  function clearActiveConversation() {
    conversationRequestId += 1;
    activeConversation.value = null;
    messages.value = [];
    isLoadingConversation.value = false;
  }

  async function loadRecentConversations(limit = DEFAULT_RECENT_CONVERSATION_LIMIT) {
    isLoadingRecent.value = true;

    try {
      recentConversations.value = await listRecentAgentConversations(limit);
    } finally {
      isLoadingRecent.value = false;
    }
  }

  async function loadConversation(conversationId: string) {
    const requestId = ++conversationRequestId;
    isLoadingConversation.value = true;

    try {
      const conversation = await getAgentConversation(conversationId);
      if (requestId !== conversationRequestId) {
        return false;
      }

      if (!conversation) {
        clearActiveConversation();
        return false;
      }

      const nextMessages = await listAgentConversationMessages(conversationId);
      if (requestId !== conversationRequestId) {
        return false;
      }

      activeConversation.value = conversation;
      messages.value = nextMessages;
      return true;
    } finally {
      if (requestId === conversationRequestId) {
        isLoadingConversation.value = false;
      }
    }
  }

  async function createConversationForAgent(agentProfileId: string) {
    isCreatingConversation.value = true;

    try {
      const conversation = await createAgentConversation(agentProfileId);
      await loadRecentConversations();
      return conversation;
    } finally {
      isCreatingConversation.value = false;
    }
  }

  async function removeConversation(conversationId: string) {
    deletingConversationId.value = conversationId;

    try {
      await deleteAgentConversation(conversationId);

      if (activeConversation.value?.id === conversationId) {
        clearActiveConversation();
      }

      recentConversations.value = recentConversations.value
        .filter(conversation => conversation.id !== conversationId);
    } finally {
      deletingConversationId.value = null;
    }
  }

  async function sendMessage(content: string) {
    const conversationId = activeConversation.value?.id;
    if (!conversationId || isSending.value) {
      return false;
    }

    isSending.value = true;

    try {
      await sendAgentConversationMessage(conversationId, content);
      await loadConversation(conversationId);
      await loadRecentConversations();
      return true;
    } finally {
      isSending.value = false;
    }
  }

  return {
    recentConversations,
    activeConversation,
    activeConversationId,
    messages,
    isLoadingRecent,
    isLoadingConversation,
    isCreatingConversation,
    isSending,
    deletingConversationId,
    clearActiveConversation,
    loadRecentConversations,
    loadConversation,
    createConversationForAgent,
    removeConversation,
    sendMessage,
  };
});

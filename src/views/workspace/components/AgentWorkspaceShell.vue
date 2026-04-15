<script setup lang="ts">
import { ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { TooltipProvider } from '@/components/ui/tooltip';
import { listAgentProfiles } from '@/services/agent-profile-storage';
import {
  listAgentConversations,
  createAgentConversation,
  deleteAgentConversation,
  listAgentConversationMessages,
  sendAgentConversationMessage,
} from '@/services/agent-conversation';
import type { AgentProfile } from '@/types';
import type { AgentWorkspaceConversation, AgentWorkspaceMessage } from '@/types/agent';
import AgentWorkspaceList from './AgentWorkspaceList.vue';
import AgentConversationList from './AgentConversationList.vue';
import AgentWorkspaceChat from './AgentWorkspaceChat.vue';

const agents = ref<AgentProfile[]>([]);
const selectedAgentId = ref<string | null>(null);
const conversations = ref<AgentWorkspaceConversation[]>([]);
const selectedConversationId = ref<string | null>(null);
const messages = ref<AgentWorkspaceMessage[]>([]);
const isLoadingAgents = ref(true);
const isLoadingConversations = ref(false);
const isSending = ref(false);

async function loadAgents() {
  isLoadingAgents.value = true;
  try {
    agents.value = await listAgentProfiles();
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '加载 Agent 失败');
  } finally {
    isLoadingAgents.value = false;
  }
}

watch(selectedAgentId, async agentId => {
  selectedConversationId.value = null;
  messages.value = [];

  if (!agentId) {
    conversations.value = [];
    return;
  }

  isLoadingConversations.value = true;
  try {
    conversations.value = await listAgentConversations(agentId);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '加载会话失败');
  } finally {
    isLoadingConversations.value = false;
  }
});

watch(selectedConversationId, async conversationId => {
  if (!conversationId) {
    messages.value = [];
    return;
  }

  try {
    messages.value = await listAgentConversationMessages(conversationId);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '加载消息失败');
  }
});

async function handleCreateConversation() {
  if (!selectedAgentId.value) {
    return;
  }

  try {
    const conversation = await createAgentConversation(selectedAgentId.value);
    conversations.value = [conversation, ...conversations.value];
    selectedConversationId.value = conversation.id;
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '创建会话失败');
  }
}

async function handleDeleteConversation(conversationId: string) {
  try {
    await deleteAgentConversation(conversationId);
    conversations.value = conversations.value.filter(c => c.id !== conversationId);
    if (selectedConversationId.value === conversationId) {
      selectedConversationId.value = null;
      messages.value = [];
    }
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '删除会话失败');
  }
}

async function handleSendMessage(content: string) {
  if (!selectedConversationId.value || isSending.value) {
    return;
  }

  isSending.value = true;
  try {
    await sendAgentConversationMessage(selectedConversationId.value, content);
    messages.value = await listAgentConversationMessages(selectedConversationId.value);
    // Refresh conversation list to update title and timestamp
    if (selectedAgentId.value) {
      conversations.value = await listAgentConversations(selectedAgentId.value);
    }
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '发送消息失败');
  } finally {
    isSending.value = false;
  }
}

// Load agents on mount
loadAgents();
</script>

<template>
  <TooltipProvider :delay-duration="0">
    <ResizablePanelGroup
      id="agent-workspace-resize-group"
      direction="horizontal"
      class="h-full items-stretch"
    >
      <!-- Left pane: Agent list -->
      <ResizablePanel
        id="agent-workspace-panel-agents"
        :default-size="22"
        :min-size="16"
        :max-size="26"
      >
        <AgentWorkspaceList
          :agents="agents"
          :selected-agent-id="selectedAgentId"
          :is-loading="isLoadingAgents"
          @select="selectedAgentId = $event"
        />
      </ResizablePanel>

      <ResizableHandle with-handle />

      <!-- Middle pane: Conversation list -->
      <ResizablePanel
        id="agent-workspace-panel-conversations"
        :default-size="30"
        :min-size="22"
        :max-size="38"
      >
        <AgentConversationList
          :conversations="conversations"
          :selected-conversation-id="selectedConversationId"
          :is-loading="isLoadingConversations"
          :has-agent-selected="!!selectedAgentId"
          @select="selectedConversationId = $event"
          @create="handleCreateConversation"
          @delete="handleDeleteConversation"
        />
      </ResizablePanel>

      <ResizableHandle with-handle />

      <!-- Right pane: Chat panel -->
      <ResizablePanel
        id="agent-workspace-panel-chat"
        :default-size="48"
        :min-size="32"
      >
        <AgentWorkspaceChat
          :messages="messages"
          :is-sending="isSending"
          :has-conversation-selected="!!selectedConversationId"
          @send="handleSendMessage"
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  </TooltipProvider>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue';
import { toast } from 'vue-sonner';
import { useRoute, useRouter } from 'vue-router';
import { useAgentWorkspaceStore } from '@/stores/agent-workspace';
import AgentWorkspaceChat from './components/AgentWorkspaceChat.vue';

const route = useRoute();
const router = useRouter();
const workspaceStore = useAgentWorkspaceStore();

const conversationId = computed(() => {
  const value = route.query.conversation;
  return typeof value === 'string' && value.trim() ? value : null;
});

const agentId = computed(() => {
  const value = route.query.agent;
  return typeof value === 'string' && value.trim() ? value : null;
});

watch([conversationId, agentId], async ([nextConversationId, nextAgentId]) => {
  if (nextConversationId) {
    try {
      const exists = await workspaceStore.loadConversation(nextConversationId);
      if (!exists) {
        toast.error('未找到这个会话');
        router.replace({ path: '/agent/workspace' });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : '加载会话失败');
    }
    return;
  }

  workspaceStore.clearActiveConversation();

  if (!nextAgentId) {
    return;
  }

  try {
    const conversation = await workspaceStore.createConversationForAgent(nextAgentId);
    router.replace({
      path: '/agent/workspace',
      query: { conversation: conversation.id },
    });
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '创建会话失败');
  }
}, { immediate: true });

async function handleSendMessage(content: string) {
  try {
    await workspaceStore.sendMessage(content);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '发送消息失败');
  }
}
</script>

<template>
  <div class="h-full bg-background">
    <AgentWorkspaceChat
      :messages="workspaceStore.messages"
      :is-loading-conversation="workspaceStore.isLoadingConversation"
      :is-sending="workspaceStore.isSending"
      :has-conversation-selected="!!workspaceStore.activeConversation"
      :conversation-title="workspaceStore.activeConversation?.title"
      :agent-name="workspaceStore.activeConversation?.snapshotAgentName"
      :work-directory="workspaceStore.activeConversation?.snapshotWorkDirectory"
      @send="handleSendMessage"
    />
  </div>
</template>

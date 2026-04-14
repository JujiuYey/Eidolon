<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import ConversationList from './components/ConversationList.vue';
import ChatPanel from './components/ChatPanel.vue';
import { sendConversationMessage } from '@/services/conversation';
import type { AgentConversation, AgentMessage } from '@/types';
import { getErrorMessage } from '@/utils/helpers';

const conversations = ref<AgentConversation[]>(createInitialConversations());
const activeConversationId = ref(conversations.value[0]?.id ?? null);
const isReplying = ref(false);

function createUserMessage(content: string): AgentMessage {
  return {
    id: `user-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role: 'user',
    content,
    createdAt: Date.now(),
    status: 'done',
  };
}

function createAssistantMessage(content: string): AgentMessage {
  return {
    id: `assistant-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role: 'assistant',
    content,
    createdAt: Date.now(),
    status: 'done',
  };
}

function createAssistantErrorMessage(content: string): AgentMessage {
  return {
    id: `assistant-error-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role: 'assistant',
    content,
    createdAt: Date.now(),
    status: 'error',
  };
}

function createConversation(title: string, messages: AgentMessage[]): AgentConversation {
  const now = Date.now();
  return {
    id: `conversation-${now}-${Math.random().toString(36).slice(2, 8)}`,
    title,
    messages,
    createdAt: now,
    updatedAt: now,
  };
}

function createInitialConversations() {
  return [
    createConversation('欢迎来到对话页', [
      createAssistantMessage('这里现在是精简后的两栏结构：左侧是对话记录，右侧是当前对话。'),
    ]),
    createConversation('产品方向讨论', [
      createUserMessage('我们先把会话页精简一下。'),
      createAssistantMessage('可以，先保留会话列表和消息区，其他上下文都去掉。'),
    ]),
    createConversation('默认模型设置', [
      createUserMessage('默认模型页先做静态，再逐步接真实数据。'),
      createAssistantMessage('已经按这个节奏推进了，结构会更稳。'),
    ]),
  ];
}

const activeConversation = computed(() =>
  conversations.value.find(conversation => conversation.id === activeConversationId.value) ?? null,
);

function updateConversation(
  conversationId: string,
  updater: (conversation: AgentConversation) => AgentConversation,
) {
  conversations.value = conversations.value.map(conversation => {
    if (conversation.id !== conversationId) {
      return conversation;
    }

    return updater(conversation);
  });
}

function handleSelectConversation(id: string) {
  activeConversationId.value = id;
}

function handleCreateConversation() {
  const conversation = createConversation('新对话', []);
  conversations.value = [conversation, ...conversations.value];
  activeConversationId.value = conversation.id;
}

function buildConversationTitle(content: string) {
  const normalized = content.replace(/\s+/g, ' ').trim();
  return normalized.length > 18 ? `${normalized.slice(0, 18)}...` : normalized;
}

async function handleSend(content: string) {
  const trimmed = content.trim();
  const currentConversation = activeConversation.value;

  if (!trimmed || !currentConversation || isReplying.value) {
    return;
  }

  const userMessage = createUserMessage(trimmed);
  const nextTitle = currentConversation.messages.length === 0
    ? buildConversationTitle(trimmed)
    : currentConversation.title;
  const nextMessages = [...currentConversation.messages, userMessage];

  updateConversation(currentConversation.id, conversation => ({
    ...conversation,
    title: nextTitle,
    updatedAt: Date.now(),
    messages: nextMessages,
  }));

  isReplying.value = true;

  try {
    const reply = await sendConversationMessage(nextMessages);

    updateConversation(currentConversation.id, conversation => ({
      ...conversation,
      updatedAt: Date.now(),
      messages: [
        ...conversation.messages,
        createAssistantMessage(reply.content),
      ],
    }));
  } catch (error) {
    const errorMessage = getErrorMessage(error, '对话失败');
    updateConversation(currentConversation.id, conversation => ({
      ...conversation,
      updatedAt: Date.now(),
      messages: [
        ...conversation.messages,
        createAssistantErrorMessage(`对话失败：${errorMessage}`),
      ],
    }));
    toast.error(errorMessage);
  } finally {
    isReplying.value = false;
  }
}
</script>

<template>
  <div class="h-screen bg-background">
    <ResizablePanelGroup direction="horizontal" class="h-full">
      <ResizablePanel :default-size="24" :min-size="18" :max-size="34">
        <div class="h-full border-r bg-muted/10">
          <ConversationList
            :conversations="conversations"
            :active-conversation-id="activeConversationId"
            @select="handleSelectConversation"
            @create="handleCreateConversation"
          />
        </div>
      </ResizablePanel>

      <ResizableHandle with-handle />

      <ResizablePanel :default-size="76" :min-size="40">
        <div class="h-full bg-background">
          <ChatPanel
            :messages="activeConversation?.messages ?? []"
            :conversation-title="activeConversation?.title ?? '新对话'"
            :busy="isReplying"
            @submit="handleSend"
          />
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  </div>
</template>

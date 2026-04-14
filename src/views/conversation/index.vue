<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import ConversationList from './components/ConversationList.vue';
import ChatPanel from './components/ChatPanel.vue';
import type { AgentConversation, AgentMessage } from '@/types';

const conversations = ref<AgentConversation[]>(createInitialConversations());
const activeConversationId = ref(conversations.value[0]?.id ?? null);
const isReplying = ref(false);
let responseTimer: number | null = null;

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

function buildMockReply(question: string, conversationTitle: string) {
  return [
    '## 本地会话回复',
    '',
    `- 当前会话：${conversationTitle}`,
    `- 当前问题：${question}`,
    '',
    '这是一个前端本地示例回复，用来演示会话列表切换和消息流交互。',
  ].join('\n');
}

function handleSend(content: string) {
  const trimmed = content.trim();
  const currentConversation = activeConversation.value;

  if (!trimmed || !currentConversation || isReplying.value) {
    return;
  }

  updateConversation(currentConversation.id, conversation => ({
    ...conversation,
    title: conversation.messages.length === 0 ? buildConversationTitle(trimmed) : conversation.title,
    updatedAt: Date.now(),
    messages: [...conversation.messages, createUserMessage(trimmed)],
  }));

  isReplying.value = true;

  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }

  responseTimer = window.setTimeout(() => {
    updateConversation(currentConversation.id, conversation => ({
      ...conversation,
      updatedAt: Date.now(),
      messages: [
        ...conversation.messages,
        createAssistantMessage(buildMockReply(trimmed, conversation.title)),
      ],
    }));
    isReplying.value = false;
    responseTimer = null;
  }, 450);
}

onBeforeUnmount(() => {
  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }
});
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

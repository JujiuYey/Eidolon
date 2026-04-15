<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import AgentConversationPanel from './components/AgentConversationPanel.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  listAgentConversationMessages,
  saveAgentConversationMessages,
} from '@/services/agent-profile';
import { getAgentProfile } from '@/services/agent-profile-storage';
import type { AgentMessage, AgentProfile } from '@/types';

const route = useRoute();
const router = useRouter();

const profile = ref<AgentProfile | null>(null);
const messages = ref<AgentMessage[]>([]);
const busy = ref(false);
const isLoading = ref(true);
let responseTimer: number | null = null;

const profileId = computed(() => String(route.params.id ?? ''));

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

function buildInitialMessage(currentProfile: AgentProfile): AgentMessage {
  return createAssistantMessage([
    `## 你好，我是 ${currentProfile.name}`,
    '',
    '当前页面还是 mock 对话模式，但已经会读取你创建这个 Agent 时选择的模型、提示词、MCP 和工具配置。',
    '',
    `- 模型：${currentProfile.modelId}`,
    `- 工作目录：${currentProfile.workDirectory || '未配置（纯聊天）'}`,
    `- MCP 服务：${currentProfile.enabledMcpServiceIds.length}`,
    '- 工具：默认启用 MCP 工具',
  ].join('\n'));
}

async function loadProfile() {
  isLoading.value = true;

  try {
    profile.value = await getAgentProfile(profileId.value);
  } finally {
    isLoading.value = false;
  }
}

function loadMessages() {
  if (!profile.value) {
    messages.value = [];
    return;
  }

  const storedMessages = listAgentConversationMessages(profile.value.id);
  messages.value = storedMessages.length > 0
    ? storedMessages
    : [buildInitialMessage(profile.value)];
}

function persistMessages() {
  if (!profile.value) {
    return;
  }

  saveAgentConversationMessages(profile.value.id, messages.value);
}

function handleBack() {
  router.push('/agent');
}

function handleSubmit(content: string) {
  const trimmed = content.trim();
  if (!trimmed || !profile.value || busy.value) {
    return;
  }

  messages.value = [...messages.value, createUserMessage(trimmed)];
  busy.value = true;
  persistMessages();

  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }

  responseTimer = window.setTimeout(() => {
    if (!profile.value) {
      return;
    }

    messages.value = [
      ...messages.value,
      createAssistantMessage(buildMockReply(profile.value, trimmed)),
    ];
    busy.value = false;
    responseTimer = null;
    persistMessages();
  }, 450);
}

function buildMockReply(currentProfile: AgentProfile, question: string) {
  return [
    `## Mock Reply`,
    '',
    `- Agent：${currentProfile.name}`,
    `- 当前问题：${question}`,
    `- 模型：${currentProfile.modelId}`,
    `- 工作目录：${currentProfile.workDirectory || '未配置（纯聊天）'}`,
    `- Temperature：${currentProfile.temperature || '未设置'}`,
    `- Max Tokens：${currentProfile.maxTokens || '未设置'}`,
    `- MCP 服务数：${currentProfile.enabledMcpServiceIds.length}`,
    '',
    `### 提示词摘要`,
    currentProfile.systemPrompt.slice(0, 220) || '未设置提示词',
    '',
    `### 模拟结论`,
    `这一页目前还不会调用真实 Agent runtime，但已经能基于你创建的 Agent 配置生成独立对话上下文。后续接入真实对话时，会优先使用这里选定的模型、MCP 服务和工具。`,
  ].join('\n');
}

watch(profileId, async () => {
  await loadProfile();
  loadMessages();
}, { immediate: true });

onBeforeUnmount(() => {
  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }
});
</script>

<template>
  <div class="h-full bg-background">
    <div v-if="isLoading" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
      <div class="text-sm text-muted-foreground">
        正在加载 Agent...
      </div>
    </div>

    <div v-else-if="!profile" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
      <Alert class="max-w-xl">
        <AlertCircle class="size-4" />
        <AlertTitle>没有找到这个 Agent</AlertTitle>
        <AlertDescription>
          这个 Agent 可能还没创建，或者已经被清理掉了。
        </AlertDescription>
        <div class="mt-4">
          <Button @click="handleBack">
            返回 Agent 列表
          </Button>
        </div>
      </Alert>
    </div>

    <AgentConversationPanel
      v-else
      :profile="profile"
      :messages="messages"
      :busy="busy"
      @back="handleBack"
      @submit="handleSubmit"
    />
  </div>
</template>

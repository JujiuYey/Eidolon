<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import AgentLayout from './components/agent-layout.vue';
import AgentConversationPanel from './components/AgentConversationPanel.vue';
import AgentProfileSummary from './components/AgentProfileSummary.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  getAgentProfile,
  listAgentConversationMessages,
  saveAgentConversationMessages,
} from '@/services/agent-profile';
import { listMcpServices } from '@/services/mcp_service';
import type { AgentMessage, AgentProfile } from '@/types';
import type { McpService } from '@/types/mcp-service';

const route = useRoute();
const router = useRouter();

const profile = ref<AgentProfile | null>(null);
const mcpServices = ref<McpService[]>([]);
const messages = ref<AgentMessage[]>([]);
const busy = ref(false);
let responseTimer: number | null = null;

const profileId = computed(() => String(route.params.id ?? ''));
const selectedToolCount = computed(() => {
  if (!profile.value) {
    return 0;
  }

  return mcpServices.value
    .filter(service => profile.value?.enabledMcpServiceIds.includes(service.id))
    .reduce((count, service) => {
      return count + (service.discovery?.tools ?? []).filter(tool => tool.enabled).length;
    }, 0);
});

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
    `- MCP 服务：${currentProfile.enabledMcpServiceIds.length}`,
    `- 工具：${selectedToolCount.value}`,
  ].join('\n'));
}

function loadProfile() {
  profile.value = getAgentProfile(profileId.value);
}

async function loadMcpServices() {
  mcpServices.value = await listMcpServices();
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

function handleEdit() {
  router.push(`/agent/${profileId.value}/edit`);
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
    `- Temperature：${currentProfile.temperature || '未设置'}`,
    `- Max Tokens：${currentProfile.maxTokens || '未设置'}`,
    `- MCP 工具数：${selectedToolCount.value}`,
    '',
    `### 提示词摘要`,
    currentProfile.systemPrompt.slice(0, 220) || '未设置提示词',
    '',
    `### 模拟结论`,
    `这一页目前还不会调用真实 Agent runtime，但已经能基于你创建的 Agent 配置生成独立对话上下文。后续接入真实对话时，会优先使用这里选定的模型、MCP 服务和工具。`,
  ].join('\n');
}

watch(profileId, async () => {
  loadProfile();
  await loadMcpServices();
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
    <div v-if="!profile" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
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

    <AgentLayout v-else>
      <template #files>
        <AgentProfileSummary
          :profile="profile"
          :mcp-services="mcpServices"
          @back="handleBack"
          @edit="handleEdit"
        />
      </template>

      <template #chat>
        <AgentConversationPanel
          :profile="profile"
          :messages="messages"
          :busy="busy"
          @submit="handleSubmit"
        />
      </template>
    </AgentLayout>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { Loader2, MessageSquare, PenSquare, Trash2 } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import { useAgentWorkspaceStore } from '@/stores/agent-workspace';

const route = useRoute();
const router = useRouter();
const workspaceStore = useAgentWorkspaceStore();

const activeConversationId = computed(() => {
  if (route.path !== '/agent/workspace') {
    return null;
  }

  const conversationId = route.query.conversation;
  return typeof conversationId === 'string' && conversationId.trim()
    ? conversationId
    : null;
});

function getConversationScopeLabel(workDirectory: string) {
  const normalized = workDirectory.trim();
  if (!normalized) {
    return '纯聊天';
  }

  return normalized.split(/[\\/]/).filter(Boolean).pop() ?? normalized;
}

function resolveCreateAgentId() {
  if (route.path === '/agent/workspace' && workspaceStore.activeConversation?.agentProfileId) {
    return workspaceStore.activeConversation.agentProfileId;
  }

  const routeAgentId = route.query.agent;
  return typeof routeAgentId === 'string' && routeAgentId.trim()
    ? routeAgentId
    : null;
}

async function loadRecentConversations() {
  try {
    await workspaceStore.loadRecentConversations();
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '加载最近会话失败');
  }
}

async function handleCreateConversation() {
  const agentId = resolveCreateAgentId();
  if (!agentId) {
    router.push('/agent');
    return;
  }

  try {
    const conversation = await workspaceStore.createConversationForAgent(agentId);
    router.push({
      path: '/agent/workspace',
      query: { conversation: conversation.id },
    });
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '创建会话失败');
  }
}

function handleSelectConversation(conversationId: string) {
  router.push({
    path: '/agent/workspace',
    query: { conversation: conversationId },
  });
}

async function handleDeleteConversation(conversationId: string) {
  try {
    await workspaceStore.removeConversation(conversationId);

    if (activeConversationId.value === conversationId) {
      router.replace({ path: '/agent/workspace' });
    }
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '删除会话失败');
  }
}

onMounted(() => {
  void loadRecentConversations();
});
</script>

<template>
  <SidebarGroup>
    <SidebarGroupLabel>最近会话</SidebarGroupLabel>
    <SidebarGroupAction
      title="新对话"
      @click="handleCreateConversation"
    >
      <PenSquare />
      <span class="sr-only">新对话</span>
    </SidebarGroupAction>

    <SidebarGroupContent>
      <SidebarMenu>
        <SidebarMenuItem
          v-for="conversation of workspaceStore.recentConversations"
          :key="conversation.id"
        >
          <SidebarMenuButton
            :is-active="conversation.id === activeConversationId"
            :tooltip="conversation.title"
            class="h-auto py-2"
            @click="handleSelectConversation(conversation.id)"
          >
            <span class="flex min-w-0 items-start gap-2">
              <MessageSquare class="mt-0.5 size-4 shrink-0" />
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm">
                  {{ conversation.title }}
                </span>
                <span class="block truncate text-xs text-muted-foreground">
                  {{ conversation.snapshotAgentName }} · {{ getConversationScopeLabel(conversation.snapshotWorkDirectory) }}
                </span>
              </span>
            </span>
          </SidebarMenuButton>

          <SidebarMenuAction
            show-on-hover
            title="删除会话"
            :disabled="workspaceStore.deletingConversationId === conversation.id"
            @click.stop="handleDeleteConversation(conversation.id)"
          >
            <Loader2
              v-if="workspaceStore.deletingConversationId === conversation.id"
              class="animate-spin"
            />
            <Trash2 v-else />
            <span class="sr-only">删除会话</span>
          </SidebarMenuAction>
        </SidebarMenuItem>
      </SidebarMenu>

      <div
        v-if="workspaceStore.isLoadingRecent"
        class="px-2 py-3 text-xs text-muted-foreground"
      >
        加载最近会话...
      </div>

      <div
        v-else-if="workspaceStore.recentConversations.length === 0"
        class="px-2 py-3 text-xs leading-5 text-muted-foreground"
      >
        还没有会话。从 Agent 列表进入后，就会在这里继续聊天。
      </div>
    </SidebarGroupContent>
  </SidebarGroup>
</template>

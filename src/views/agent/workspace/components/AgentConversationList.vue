<script setup lang="ts">
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';
import { MessageSquare, Plus, Trash2 } from 'lucide-vue-next';
import type { AgentWorkspaceConversation } from '@/types/agent';

defineProps<{
  conversations: AgentWorkspaceConversation[];
  selectedConversationId: string | null;
  isLoading: boolean;
  hasAgentSelected: boolean;
}>();

const emit = defineEmits<{
  select: [conversationId: string];
  create: [];
  delete: [conversationId: string];
}>();
</script>

<template>
  <div class="flex h-full flex-col border-r bg-sidebar">
    <!-- Header -->
    <div class="flex items-center justify-between border-b px-4 py-3">
      <div class="flex items-center gap-2">
        <MessageSquare class="size-5 text-primary" />
        <span class="font-semibold">会话</span>
      </div>
      <button
        type="button"
        class="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
        :disabled="!hasAgentSelected"
        title="新建会话"
        @click="emit('create')"
      >
        <Plus class="size-4" />
      </button>
    </div>

    <!-- Loading state -->
    <div
      v-if="isLoading"
      class="flex flex-1 items-center justify-center"
    >
      <div class="text-sm text-muted-foreground">
        加载中...
      </div>
    </div>

    <!-- Empty state: no agent selected -->
    <div
      v-else-if="!hasAgentSelected"
      class="flex flex-1 items-center justify-center p-4"
    >
      <div class="text-center text-sm text-muted-foreground">
        请先选择一个 Agent
      </div>
    </div>

    <!-- Empty state: agent selected but no conversations -->
    <div
      v-else-if="conversations.length === 0"
      class="flex flex-1 items-center justify-center p-4"
    >
      <div class="text-center text-sm text-muted-foreground">
        还没有会话，点击 + 创建
      </div>
    </div>

    <!-- Conversation list -->
    <div v-else class="flex-1 overflow-y-auto">
      <div
        v-for="conversation of conversations"
        :key="conversation.id"
        class="group flex items-center gap-2 border-b px-4 py-3 transition-colors"
        :class="[
          selectedConversationId === conversation.id
            ? 'bg-accent'
            : 'hover:bg-accent/50',
        ]"
      >
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-3 text-left"
          @click="emit('select', conversation.id)"
        >
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium">
              {{ conversation.title }}
            </div>
            <div class="truncate text-xs text-muted-foreground">
              {{ formatDistanceToNow(new Date(conversation.updatedAt), { locale: zhCN, addSuffix: true }) }}
            </div>
          </div>
        </button>

        <button
          type="button"
          class="flex size-8 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
          title="删除会话"
          @click.stop="emit('delete', conversation.id)"
        >
          <Trash2 class="size-4" />
        </button>
      </div>
    </div>
  </div>
</template>

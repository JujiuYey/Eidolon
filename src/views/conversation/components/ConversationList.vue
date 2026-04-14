<script setup lang="ts">
import { MessageSquarePlus, MessageSquareText } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { AgentConversation } from '@/types';

const props = defineProps<{
  conversations: AgentConversation[];
  activeConversationId: string | null;
}>();

const emit = defineEmits<{
  (e: 'select', id: string): void;
  (e: 'create'): void;
}>();

function formatUpdatedAt(timestamp: number) {
  return new Intl.DateTimeFormat('zh-CN', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(timestamp);
}

function getPreview(conversation: AgentConversation) {
  const lastMessage = [...conversation.messages].reverse().find(message => message.content.trim());
  if (!lastMessage) {
    return '开始新的对话';
  }

  return lastMessage.content.replace(/\s+/g, ' ').trim();
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="border-b bg-background px-4 py-4">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <MessageSquareText class="h-4 w-4 text-primary" />
          <span class="text-sm font-medium">对话记录</span>
        </div>

        <Button
          type="button"
          size="sm"
          class="h-8 rounded-full px-3"
          @click="emit('create')"
        >
          <MessageSquarePlus class="h-4 w-4" />
          新对话
        </Button>
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-2 p-3">
        <button
          v-for="conversation of props.conversations"
          :key="conversation.id"
          type="button"
          class="w-full rounded-2xl border px-4 py-3 text-left transition-colors"
          :class="conversation.id === props.activeConversationId
            ? 'border-primary bg-primary/5'
            : 'border-transparent bg-background hover:bg-muted/40'"
          @click="emit('select', conversation.id)"
        >
          <div class="flex items-start justify-between gap-3">
            <p class="line-clamp-1 text-sm font-medium text-foreground">
              {{ conversation.title }}
            </p>
            <span class="shrink-0 text-[11px] text-muted-foreground">
              {{ formatUpdatedAt(conversation.updatedAt) }}
            </span>
          </div>

          <p class="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">
            {{ getPreview(conversation) }}
          </p>
        </button>

        <div
          v-if="props.conversations.length === 0"
          class="rounded-2xl border border-dashed px-4 py-8 text-center text-sm text-muted-foreground"
        >
          还没有对话，点击“新对话”开始。
        </div>
      </div>
    </ScrollArea>
  </div>
</template>

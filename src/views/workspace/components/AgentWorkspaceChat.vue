<script setup lang="ts">
import { ref } from 'vue';
import { Send, Loader2, PanelLeft } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { AgentWorkspaceMessage } from '@/types/agent';

defineProps<{
  messages: AgentWorkspaceMessage[];
  isLoadingConversation: boolean;
  isSending: boolean;
  hasConversationSelected: boolean;
  conversationTitle?: string;
  agentName?: string;
}>();

const emit = defineEmits<{
  send: [content: string];
}>();

const inputContent = ref('');

function handleSend() {
  const content = inputContent.value.trim();
  if (!content) {
    return;
  }

  emit('send', content);
  inputContent.value = '';
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    handleSend();
  }
}
</script>

<template>
  <div class="flex h-full flex-col bg-background">
    <div
      v-if="hasConversationSelected"
      class="border-b px-5 py-4"
    >
      <div class="min-w-0">
        <div class="truncate text-sm font-medium">
          {{ conversationTitle || '新对话' }}
        </div>
        <p class="truncate text-xs text-muted-foreground">
          {{ agentName || '当前会话' }}
        </p>
      </div>
    </div>

    <div
      v-if="isLoadingConversation"
      class="flex flex-1 items-center justify-center"
    >
      <div class="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        加载会话中...
      </div>
    </div>

    <div
      v-else-if="!hasConversationSelected"
      class="flex flex-1 items-center justify-center"
    >
      <div class="text-center">
        <PanelLeft class="mx-auto mb-4 size-12 text-muted-foreground/50" />
        <div class="text-sm text-muted-foreground">
          从左侧最近会话继续聊天，或先到 Agent 列表发起一个新对话
        </div>
      </div>
    </div>

    <template v-else>
      <ScrollArea class="flex-1">
        <div class="mx-auto flex max-w-4xl flex-col gap-4 p-5">
          <div
            v-if="messages.length === 0"
            class="flex flex-1 items-center justify-center py-12"
          >
            <div class="text-center text-sm text-muted-foreground">
              开始发送消息与 Agent 对话
            </div>
          </div>

          <div
            v-for="message of messages"
            :key="message.id"
            class="flex gap-3"
            :class="message.role === 'user' ? 'flex-row-reverse' : 'flex-row'"
          >
            <div
              class="max-w-[80%] rounded-lg px-4 py-2"
              :class="[
                message.role === 'user'
                  ? 'bg-primary text-primary-foreground'
                  : message.status === 'error'
                    ? 'bg-destructive/10 text-destructive'
                    : 'bg-muted',
              ]"
            >
              <div class="mb-1 text-[11px] text-muted-foreground/80">
                {{ message.role === 'user' ? '你' : '助手' }}
              </div>
              <div class="whitespace-pre-wrap text-sm">
                {{ message.content }}
              </div>
            </div>
          </div>
        </div>
      </ScrollArea>

      <div class="border-t p-4">
        <div class="mx-auto flex max-w-4xl items-end gap-2">
          <textarea
            v-model="inputContent"
            class="flex-1 resize-none rounded-lg border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50"
            placeholder="输入消息..."
            rows="1"
            :disabled="isSending || isLoadingConversation || !hasConversationSelected"
            @keydown="handleKeydown"
          />
          <Button
            size="icon"
            :disabled="isSending || isLoadingConversation || !inputContent.trim() || !hasConversationSelected"
            @click="handleSend"
          >
            <Loader2 v-if="isSending" class="size-4 animate-spin" />
            <Send v-else class="size-4" />
          </Button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { Send, Loader2, MessageSquare } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { AgentWorkspaceMessage } from '@/types/agent';

defineProps<{
  messages: AgentWorkspaceMessage[];
  isSending: boolean;
  hasConversationSelected: boolean;
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
    <!-- Empty state: no conversation selected -->
    <div
      v-if="!hasConversationSelected"
      class="flex flex-1 items-center justify-center"
    >
      <div class="text-center">
        <MessageSquare class="mx-auto mb-4 size-12 text-muted-foreground/50" />
        <div class="text-sm text-muted-foreground">
          请选择一个会话开始对话
        </div>
      </div>
    </div>

    <!-- Chat content -->
    <template v-else>
      <!-- Messages -->
      <ScrollArea class="flex-1">
        <div class="flex flex-col gap-4 p-4">
          <!-- Empty conversation state -->
          <div
            v-if="messages.length === 0"
            class="flex flex-1 items-center justify-center py-8"
          >
            <div class="text-center text-sm text-muted-foreground">
              开始发送消息与 Agent 对话
            </div>
          </div>

          <!-- Message list -->
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
              <div class="whitespace-pre-wrap text-sm">
                {{ message.content }}
              </div>
            </div>
          </div>
        </div>
      </ScrollArea>

      <!-- Input area -->
      <div class="border-t p-4">
        <div class="flex items-end gap-2">
          <textarea
            v-model="inputContent"
            class="flex-1 resize-none rounded-lg border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50"
            placeholder="输入消息..."
            rows="1"
            :disabled="isSending"
            @keydown="handleKeydown"
          />
          <Button
            size="icon"
            :disabled="isSending || !inputContent.trim()"
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

<script setup lang="ts">
import type { AgentMessage } from '@/types';
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from '@/components/ai-elements/conversation';
import {
  Message,
  MessageContent,
  MessageResponse,
} from '@/components/ai-elements/message';
import {
  PromptInput,
  PromptInputProvider,
  PromptInputBody,
  PromptInputTextarea,
  PromptInputSubmit,
  PromptInputFooter,
} from '@/components/ai-elements/prompt-input';

interface Props {
  messages?: AgentMessage[];
  conversationTitle?: string;
  busy: boolean;
}

interface Emits {
  (e: 'submit', value: string): void;
}

const props = withDefaults(defineProps<Props>(), {
  messages: () => [],
  conversationTitle: '新对话',
});

const emit = defineEmits<Emits>();

function handleSubmit(payload: { text: string; files: unknown[] }) {
  if (payload.text.trim()) {
    emit('submit', payload.text.trim());
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b bg-background px-4 py-3">
      <div class="min-w-0">
        <div class="text-sm font-medium">
          {{ props.conversationTitle }}
        </div>
        <p class="text-xs text-muted-foreground">
          当前会话的消息内容会展示在这里。
        </p>
      </div>
    </div>

    <Conversation class="min-h-0">
      <ConversationEmptyState
        v-if="messages.length === 0"
        title="开始新的对话"
        description="在下方输入消息，开始一段新的会话。"
      />

      <ConversationContent v-else class="gap-4">
        <Message
          v-for="message of messages"
          :key="message.id"
          :from="message.role"
          :class="message.role === 'user' ? undefined : 'max-w-full'"
        >
          <MessageContent
            :class="message.role === 'user'
              ? 'max-w-full'
              : 'w-full rounded-lg border bg-background p-4'"
          >
            <div class="mb-2 text-xs text-muted-foreground">
              {{ message.role === 'user' ? '你' : '助手' }}
            </div>

            <MessageResponse
              :content="message.content"
              class="whitespace-pre-wrap break-words [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
            />
          </MessageContent>
        </Message>
      </ConversationContent>

      <ConversationScrollButton />
    </Conversation>

    <div class="border-t bg-background p-4">
      <PromptInputProvider @submit="handleSubmit">
        <PromptInput>
          <PromptInputBody>
            <PromptInputTextarea placeholder="输入消息，开始对话" />
          </PromptInputBody>
          <PromptInputFooter class="justify-end">
            <PromptInputSubmit :status="busy ? 'submitted' : undefined" />
          </PromptInputFooter>
        </PromptInput>
      </PromptInputProvider>
    </div>
  </div>
</template>

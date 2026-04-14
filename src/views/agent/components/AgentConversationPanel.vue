<script setup lang="ts">
import { ArrowLeft } from 'lucide-vue-next';
import type { AgentMessage, AgentProfile } from '@/types';
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
  PromptInputBody,
  PromptInputFooter,
  PromptInputProvider,
  PromptInputSubmit,
  PromptInputTextarea,
} from '@/components/ai-elements/prompt-input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

defineProps<{
  profile: AgentProfile;
  messages: AgentMessage[];
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'submit', value: string): void;
}>();

function handleSubmit(payload: { text: string; files: unknown[] }) {
  if (payload.text.trim()) {
    emit('submit', payload.text.trim());
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b bg-background px-4 py-4">
      <div class="flex flex-col gap-3">
        <div>
          <Button variant="ghost" size="sm" class="px-0" @click="emit('back')">
            <ArrowLeft class="size-4" />
            返回 Agent 列表
          </Button>
        </div>

        <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div class="min-w-0">
            <div class="text-sm font-medium">
              {{ profile.name }}
            </div>
            <p class="text-xs text-muted-foreground">
              当前仍是 mock 对话页，后续会把真实模型调用、MCP 和工具执行接进来。
            </p>
          </div>

          <div class="flex flex-wrap gap-2">
            <Badge variant="outline">
              {{ profile.modelId }}
            </Badge>
            <Badge variant="outline">
              {{ profile.enabledMcpServiceIds.length }} MCP
            </Badge>
            <Badge variant="outline">
              默认启用 MCP 工具
            </Badge>
          </div>
        </div>
      </div>
    </div>

    <Conversation class="min-h-0">
      <ConversationEmptyState
        v-if="messages.length === 0"
        title="开始新的 Agent 对话"
        description="发送第一条消息，体验这个 Agent 的独立对话入口。"
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
              {{ message.role === 'user' ? '你' : profile.name }}
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
            <PromptInputTextarea :placeholder="`向 ${profile.name} 发送消息`" />
          </PromptInputBody>
          <PromptInputFooter class="justify-end">
            <PromptInputSubmit :status="busy ? 'submitted' : undefined" />
          </PromptInputFooter>
        </PromptInput>
      </PromptInputProvider>
    </div>
  </div>
</template>

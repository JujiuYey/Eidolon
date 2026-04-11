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
import { Badge } from '@/components/ui/badge';

interface Props {
  messages?: AgentMessage[];
  selectedFilePath?: string | null;
  selectedFileContent?: string;
  busy: boolean;
}

interface Emits {
  (e: 'submit', value: string): void;
}

const props = withDefaults(defineProps<Props>(), {
  messages: () => [],
  selectedFilePath: null,
  selectedFileContent: '',
});

const emit = defineEmits<Emits>();

const filePreview = computed(() => {
  if (!props.selectedFileContent.trim()) {
    return '当前没有可用的 mock 文件内容';
  }

  return props.selectedFileContent.split('\n').slice(0, 8).join('\n');
});

function handleSubmit(payload: { text: string; files: unknown[] }) {
  if (payload.text.trim()) {
    emit('submit', payload.text.trim());
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b bg-background px-4 py-3">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div class="min-w-0">
          <div class="text-sm font-medium">
            代码分析对话
          </div>
          <p class="text-xs text-muted-foreground">
            当前页面已切换为 mock 模式，不依赖真实 Agent 档案、模型调用或后端执行过程。
          </p>
        </div>

        <Badge variant="outline" class="w-fit">
          Mock Mode
        </Badge>
      </div>

      <div class="mt-4 rounded-lg border bg-muted/20 p-3">
        <p class="text-xs font-medium text-muted-foreground">
          当前文件上下文
        </p>
        <p class="mt-1 text-sm font-medium break-all">
          {{ selectedFilePath || '未选中文件' }}
        </p>
        <pre class="mt-3 max-h-40 overflow-auto whitespace-pre-wrap rounded-md bg-background p-3 text-xs leading-5">{{ filePreview }}</pre>
      </div>
    </div>

    <Conversation class="min-h-0">
      <ConversationEmptyState
        v-if="messages.length === 0"
        title="欢迎使用 Mock 代码分析对话"
        description="发送消息开始体验前端假数据对话。"
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
              {{ message.role === 'user' ? '你' : 'Mock AI' }}
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
            <PromptInputTextarea placeholder="输入一个代码分析问题，当前会返回 mock 结果" />
          </PromptInputBody>
          <PromptInputFooter class="justify-end">
            <PromptInputSubmit :status="busy ? 'submitted' : undefined" />
          </PromptInputFooter>
        </PromptInput>
      </PromptInputProvider>
    </div>
  </div>
</template>

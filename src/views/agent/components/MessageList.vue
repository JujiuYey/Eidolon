<script setup lang="ts">
import { Bot, User2 } from 'lucide-vue-next';
import ToolTimeline from './ToolTimeline.vue';
import type { AgentMessage } from '@/types';

interface Props {
  messages: AgentMessage[];
  busy: boolean;
}

defineProps<Props>();
</script>

<template>
  <ScrollArea class="h-full">
    <div class="space-y-4 p-4">
      <div v-if="messages.length === 0" class="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
        选择一个会话，或者直接开始新的代码分析问题。
      </div>

      <div v-for="message of messages" :key="message.id" class="flex gap-3">
        <div class="mt-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-full border bg-background">
          <User2 v-if="message.role === 'user'" class="h-4 w-4" />
          <Bot v-else class="h-4 w-4 text-primary" />
        </div>

        <div class="min-w-0 flex-1">
          <div class="mb-1 flex items-center gap-2 text-xs text-muted-foreground">
            <span>{{ message.role === 'user' ? '你' : '代码分析 Agent' }}</span>
            <Badge v-if="message.status && message.role === 'assistant'" variant="outline">
              {{ message.status }}
            </Badge>
          </div>

          <div class="rounded-xl border bg-card p-4">
            <div class="whitespace-pre-wrap break-words text-sm leading-6">
              {{ message.content || (busy && message.role === 'assistant' ? '正在分析项目...' : '') }}
            </div>

            <ToolTimeline :traces="message.toolTraces" />
          </div>
        </div>
      </div>
    </div>
  </ScrollArea>
</template>

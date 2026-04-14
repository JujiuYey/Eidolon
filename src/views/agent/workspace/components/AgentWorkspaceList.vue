<script setup lang="ts">
import { Bot } from 'lucide-vue-next';
import type { AgentProfile } from '@/types';

defineProps<{
  agents: AgentProfile[];
  selectedAgentId: string | null;
  isLoading: boolean;
}>();

const emit = defineEmits<{
  select: [agentId: string];
}>();
</script>

<template>
  <div class="flex h-full flex-col border-r bg-sidebar">
    <!-- Header -->
    <div class="flex items-center gap-2 border-b px-4 py-3">
      <Bot class="size-5 text-primary" />
      <span class="font-semibold">Agent</span>
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

    <!-- Empty state -->
    <div
      v-else-if="agents.length === 0"
      class="flex flex-1 items-center justify-center p-4"
    >
      <div class="text-center text-sm text-muted-foreground">
        还没有 Agent
      </div>
    </div>

    <!-- Agent list -->
    <div v-else class="flex-1 overflow-y-auto">
      <button
        v-for="agent of agents"
        :key="agent.id"
        type="button"
        class="flex w-full items-center gap-3 border-b px-4 py-3 text-left transition-colors hover:bg-accent"
        :class="[
          selectedAgentId === agent.id
            ? 'bg-accent'
            : 'bg-transparent',
        ]"
        @click="emit('select', agent.id)"
      >
        <div class="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary/10">
          <Bot class="size-4 text-primary" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-medium">
            {{ agent.name }}
          </div>
          <div class="truncate text-xs text-muted-foreground">
            {{ agent.modelId }}
          </div>
        </div>
      </button>
    </div>
  </div>
</template>

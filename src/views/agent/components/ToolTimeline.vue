<script setup lang="ts">
import { reactive } from 'vue';
import { ChevronRight, Wrench } from 'lucide-vue-next';
import { cn } from '@/lib/utils';
import type { AgentToolTrace } from '@/types';

interface Props {
  traces?: AgentToolTrace[];
}

const props = defineProps<Props>();
const openState = reactive<Record<string, boolean>>({});

function isOpen(trace: AgentToolTrace) {
  return openState[trace.id] ?? (trace.status === 'running' || trace.status === 'error');
}

function toggle(traceId: string) {
  openState[traceId] = !(openState[traceId] ?? false);
}
</script>

<template>
  <div v-if="props.traces?.length" class="mt-3 rounded-lg border bg-muted/30 p-3">
    <div class="mb-3 flex items-center gap-2 text-sm font-medium">
      <Wrench class="h-4 w-4 text-primary" />
      <span>工具轨迹</span>
    </div>

    <div class="space-y-3">
      <div v-for="trace of props.traces" :key="trace.id" class="rounded-md border bg-background">
        <button
          type="button"
          class="flex w-full items-center justify-between gap-3 p-3 text-left"
          @click="toggle(trace.id)"
        >
          <div class="flex min-w-0 items-center gap-2">
            <ChevronRight
              class="h-4 w-4 shrink-0 transition-transform"
              :class="cn(isOpen(trace) && 'rotate-90')"
            />
            <span class="truncate text-sm font-medium">{{ trace.name }}</span>
          </div>

          <Badge variant="secondary" :class="trace.status === 'error' ? 'text-red-600' : ''">
            {{ trace.status }}
          </Badge>
        </button>

        <div v-if="isOpen(trace)" class="space-y-2 border-t px-3 pb-3 pt-2 text-xs">
          <div>
            <div class="mb-1 text-muted-foreground">
              参数
            </div>
            <pre class="overflow-x-auto whitespace-pre-wrap break-all rounded bg-muted p-2">{{ trace.argsText }}</pre>
          </div>

          <div v-if="trace.resultText">
            <div class="mb-1 text-muted-foreground">
              结果
            </div>
            <pre class="overflow-x-auto whitespace-pre-wrap break-all rounded bg-muted p-2">{{ trace.resultText }}</pre>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

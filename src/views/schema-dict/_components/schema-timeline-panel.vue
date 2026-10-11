<script setup lang="ts">
import { Loader2, MessageCircleQuestion, Sparkles } from 'lucide-vue-next';
import { computed } from 'vue';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { TimelineEntry } from '@/types/schema-dict';

import { changeKindLabel, changeTone, CHANGE_TONE_CLASSES, describeChange, sortChanges } from '../utils/display';

const props = defineProps<{
  entries: TimelineEntry[];
  loading: boolean;
  interpret: { status: 'loading' | 'ready' | 'error'; text: string; errorMessage: string } | null;
}>();

const emit = defineEmits<{
  (e: 'interpret', payload: { beforeId: string; afterId: string }): void;
  (e: 'dismiss-interpret'): void;
}>();

/** 时间线按时间倒序展示（最新在上）；后端按 created_at 升序返回 */
const orderedEntries = computed(() => [...props.entries].reverse());

function formatTime(ms: number): string {
  if (!ms) {
    return '-';
  }
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function previousId(index: number): string | null {
  const originalIndex = orderedEntries.value.length - 1 - index;
  return originalIndex > 0 ? (props.entries[originalIndex - 1]?.snapshot_id ?? null) : null;
}
</script>

<template>
  <div class="space-y-3">
    <!-- AI 解读结果 -->
    <div v-if="interpret" class="rounded-xl border border-primary/30 bg-primary/5 p-4">
      <div class="flex items-center gap-2">
        <Sparkles class="size-4 text-primary" />
        <h3 class="text-sm font-semibold">变更解读</h3>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="ml-auto text-muted-foreground"
          @click="emit('dismiss-interpret')"
        >
          关闭
        </Button>
      </div>
      <div v-if="interpret.status === 'loading'" class="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在解读…
      </div>
      <p v-else-if="interpret.status === 'error'" class="mt-3 text-sm text-destructive">{{ interpret.errorMessage }}</p>
      <p v-else class="mt-3 whitespace-pre-wrap text-sm leading-6">{{ interpret.text }}</p>
    </div>

    <div v-if="loading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
      <Loader2 class="size-4 animate-spin" />
      正在加载时间线…
    </div>

    <div v-else-if="orderedEntries.length === 0" class="py-10 text-center text-sm text-muted-foreground">
      还没有快照，先在页头导入一份表结构
    </div>

    <template v-else>
      <article v-for="(entry, index) of orderedEntries" :key="entry.snapshot_id" class="rounded-xl border bg-card p-4">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="text-sm font-semibold">{{ entry.label }}</h3>
          <Badge v-if="entry.source_kind === 'query'" variant="secondary" class="text-xs">查询导入</Badge>
          <span class="text-xs text-muted-foreground tabular-nums">{{ formatTime(entry.created_at) }}</span>
          <Button
            v-if="previousId(index) && entry.changes.length > 0"
            type="button"
            variant="outline"
            size="sm"
            class="ml-auto"
            :disabled="interpret?.status === 'loading'"
            @click="emit('interpret', { beforeId: previousId(index)!, afterId: entry.snapshot_id })"
          >
            <MessageCircleQuestion class="size-3.5" />
            AI 解读
          </Button>
        </div>

        <div class="mt-3 space-y-1.5">
          <p v-if="index === orderedEntries.length - 1" class="text-xs text-muted-foreground">首次导入，没有对比基准</p>
          <p v-else-if="entry.changes.length === 0" class="text-xs text-muted-foreground">与上一版结构一致，没有变化</p>
          <div
            v-for="(change, changeIndex) of sortChanges(entry.changes)"
            :key="`${change.kind}-${change.table}-${change.column ?? ''}-${changeIndex}`"
            class="flex items-start gap-2 text-sm"
          >
            <Badge
              variant="secondary"
              class="mt-0.5 shrink-0 text-xs"
              :class="CHANGE_TONE_CLASSES[changeTone(change.kind)]"
            >
              {{ changeKindLabel(change.kind) }}
            </Badge>
            <span class="min-w-0 break-all leading-5">{{ describeChange(change) }}</span>
          </div>
        </div>
      </article>
    </template>
  </div>
</template>

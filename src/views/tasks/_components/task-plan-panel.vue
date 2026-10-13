<script setup lang="ts">
import { CalendarCheck, Check, Loader2, RotateCcw, X } from 'lucide-vue-next';
import { computed } from 'vue';

import { Button } from '@/components/ui/button';
import type { AiPlanState } from '@/stores/tasks';
import type { TaskPriority } from '@/types/tasks';

import { priorityDotClass } from '../utils/display';
import { formatMinutes, parseDatetime } from '../utils/schedule';

/**
 * AI 排期建议面板：按天分组的时间块预览 + 一键应用。
 * AI 永远只建议——「应用到日程」才写库，失败可重试。
 */

const props = defineProps<{
  plan: AiPlanState;
}>();

const emit = defineEmits<{
  (e: 'dismiss'): void;
  (e: 'retry'): void;
  (e: 'apply'): void;
}>();

const WEEKDAY_LABELS = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

interface PreviewBlock {
  key: string;
  title: string;
  priority: TaskPriority;
  range: string;
}

const grouped = computed<Array<{ key: string; label: string; blocks: PreviewBlock[] }>>(() => {
  const groups = new Map<string, { key: string; label: string; blocks: PreviewBlock[] }>();
  props.plan.blocks.forEach((block, index) => {
    const start = parseDatetime(block.start_at);
    const end = parseDatetime(block.end_at);
    if (!start || !end) {
      return;
    }
    const weekday = WEEKDAY_LABELS[new Date(`${start.date}T00:00`).getDay()] ?? '';
    const label = `${weekday} ${start.date.slice(5).replace('-', '/')}`;
    if (!groups.has(start.date)) {
      groups.set(start.date, { key: start.date, label, blocks: [] });
    }
    groups.get(start.date)!.blocks.push({
      key: `${block.task_id}_${index}`,
      title: block.title,
      priority: block.priority,
      range: `${formatMinutes(start.minutes)}–${formatMinutes(end.minutes)}`,
    });
  });
  return [...groups.values()].sort((a, b) => a.key.localeCompare(b.key));
});
</script>

<template>
  <section class="shrink-0 rounded-xl border border-primary/30 bg-primary/[0.04] p-4" aria-live="polite">
    <div class="flex items-center gap-2">
      <CalendarCheck class="size-4 shrink-0 text-primary" />
      <h2 class="shrink-0 text-sm font-semibold">排期建议</h2>
      <span v-if="plan.status === 'ready' && plan.note" class="truncate text-xs text-muted-foreground">
        · {{ plan.note }}
      </span>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        class="ml-auto size-7 shrink-0 text-muted-foreground"
        aria-label="关闭建议"
        @click="emit('dismiss')"
      >
        <X class="size-4" />
      </Button>
    </div>

    <div v-if="plan.status === 'loading'" class="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
      <Loader2 class="size-4 animate-spin" />
      正在排…
    </div>

    <div v-else-if="plan.status === 'error'" class="mt-3 space-y-2">
      <p class="text-sm text-destructive">{{ plan.errorMessage }}</p>
      <Button type="button" variant="outline" size="sm" @click="emit('retry')"> 重试 </Button>
    </div>

    <template v-else>
      <div class="mt-3 space-y-3">
        <div v-for="group of grouped" :key="group.key">
          <p class="text-xs font-medium text-muted-foreground">{{ group.label }}</p>
          <ul class="mt-1 space-y-1">
            <li
              v-for="block of group.blocks"
              :key="block.key"
              class="flex items-center gap-2 rounded-md border border-border/60 bg-card px-2.5 py-1.5 text-xs"
            >
              <span class="w-24 shrink-0 tabular-nums text-muted-foreground">{{ block.range }}</span>
              <span
                v-if="block.priority !== 'none'"
                class="size-1.5 shrink-0 rounded-full"
                :class="priorityDotClass(block.priority)"
                aria-hidden="true"
              />
              <span class="truncate font-medium">{{ block.title }}</span>
            </li>
          </ul>
        </div>
      </div>

      <div class="mt-3 flex items-center justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" @click="emit('retry')">
          <RotateCcw class="size-3.5" />
          重新排
        </Button>
        <Button type="button" size="sm" @click="emit('apply')">
          <Check class="size-3.5" />
          应用到日程
        </Button>
      </div>
    </template>
  </section>
</template>

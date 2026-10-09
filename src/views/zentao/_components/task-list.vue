<script setup lang="ts">
import type { ZentaoTask } from '@/types/zentao';
import { computed } from 'vue';
import { AlarmClock, CircleCheck, CircleDashed, CircleDotDashed } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import {
  formatHours,
  isOverdue,
  priLabel,
  sortTasks,
  taskStatusLabel,
} from '../utils/display';

const props = defineProps<{
  tasks: ZentaoTask[];
}>();

const sorted = computed(() => sortTasks(props.tasks));

/** 优先级 → 卡片左缘色条颜色（P1 红 / P2 橙 / P3 蓝 / 其余灰） */
function priBarClass(pri: number): string {
  if (pri <= 1) {
    return 'bg-destructive';
  }
  if (pri === 2) {
    return 'bg-orange-500';
  }
  if (pri === 3) {
    return 'bg-sky-500';
  }
  return 'bg-muted-foreground/40';
}

function statusIcon(status: string) {
  if (status === 'doing') {
    return CircleDotDashed;
  }
  if (status === 'pause') {
    return CircleDashed;
  }
  return CircleCheck;
}
</script>

<template>
  <div class="grid gap-3 md:grid-cols-2">
    <article
      v-for="task of sorted"
      :key="task.id"
      class="group relative overflow-hidden rounded-lg border bg-card p-4 pl-5 transition-colors hover:border-ring/40 hover:bg-muted/30"
    >
      <!-- 优先级色条 -->
      <span
        class="absolute inset-y-0 left-0 w-1.5"
        :class="priBarClass(task.pri)"
        aria-hidden="true"
      />

      <div class="flex items-start justify-between gap-3">
        <h3 class="line-clamp-2 min-w-0 text-sm leading-6 font-medium">
          {{ task.name }}
        </h3>
        <Badge
          variant="secondary"
          class="shrink-0"
        >
          {{ priLabel(task.pri) }}
        </Badge>
      </div>

      <p
        v-if="task.story_title"
        class="mt-1 line-clamp-1 text-xs text-muted-foreground"
      >
        {{ task.story_title }}
      </p>

      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted-foreground">
        <span class="inline-flex items-center gap-1">
          <component
            :is="statusIcon(task.status)"
            class="size-3.5"
            :class="task.status === 'doing' ? 'text-primary' : ''"
          />
          {{ taskStatusLabel(task.status) }}
        </span>

        <span
          v-if="task.deadline"
          class="inline-flex items-center gap-1 tabular-nums"
          :class="isOverdue(task.deadline) ? 'font-medium text-destructive' : ''"
        >
          <AlarmClock class="size-3.5" />
          {{ task.deadline }}
          <template v-if="isOverdue(task.deadline)">已逾期</template>
        </span>

        <span class="tabular-nums">
          剩余 {{ formatHours(task.left) }}h / 预估 {{ formatHours(task.estimate) }}h
        </span>
      </div>

      <footer
        v-if="task.execution_name"
        class="mt-3 border-t pt-2.5 text-xs text-muted-foreground"
      >
        {{ task.execution_name }}
      </footer>
    </article>
  </div>
</template>

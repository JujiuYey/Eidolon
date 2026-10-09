<script setup lang="ts">
import type { ZentaoTask } from '@/types/zentao';
import { computed } from 'vue';
import { AlarmClock } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import {
  formatHours,
  isOverdue,
  priLabel,
  priVariant,
  sortTasks,
  taskStatusLabel,
  taskStatusVariant,
} from '../utils/display';

const props = defineProps<{
  tasks: ZentaoTask[];
}>();

const sorted = computed(() => sortTasks(props.tasks));
</script>

<template>
  <div class="flex flex-col divide-y">
    <div
      v-for="task of sorted"
      :key="task.id"
      class="flex flex-wrap items-center gap-x-3 gap-y-1 px-1 py-3 transition-colors hover:bg-muted/50"
    >
      <Badge :variant="priVariant(task.pri)">
        {{ priLabel(task.pri) }}
      </Badge>

      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium">
          {{ task.name }}
        </p>
        <p
          v-if="task.story_title || task.execution_name"
          class="truncate text-xs text-muted-foreground"
        >
          {{ task.execution_name }}<template v-if="task.story_title">
            · {{ task.story_title }}
          </template>
        </p>
      </div>

      <div class="text-right text-xs tabular-nums text-muted-foreground">
        预估 {{ formatHours(task.estimate) }} · 剩余 {{ formatHours(task.left) }} · 消耗 {{ formatHours(task.consumed) }}
      </div>

      <div
        v-if="task.deadline"
        class="flex items-center gap-1 text-xs tabular-nums"
        :class="isOverdue(task.deadline) ? 'font-medium text-destructive' : 'text-muted-foreground'"
      >
        <AlarmClock class="h-3.5 w-3.5" />
        {{ task.deadline }}
        <span v-if="isOverdue(task.deadline)">逾期</span>
      </div>

      <Badge :variant="taskStatusVariant(task.status)">
        {{ taskStatusLabel(task.status) }}
      </Badge>
    </div>
  </div>
</template>

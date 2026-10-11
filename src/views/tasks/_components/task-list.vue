<script setup lang="ts">
import {
  AlarmClock,
  Circle,
  CircleCheck,
  CircleDotDashed,
  FileText,
  MoreHorizontal,
  Pencil,
  Trash2,
} from 'lucide-vue-next';
import { computed } from 'vue';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { Task } from '@/types/tasks';

import { isDueToday, isOverdue, priorityDotClass, sortTasks, taskPriorityLabel } from '../utils/display';

const props = defineProps<{
  tasks: Task[];
}>();

const emit = defineEmits<{
  (e: 'toggle', task: Task): void;
  (e: 'edit', task: Task): void;
  (e: 'remove', task: Task): void;
}>();

const sorted = computed(() => sortTasks(props.tasks));

function statusIconClass(task: Task): string {
  if (task.status === 'done') {
    return 'text-primary';
  }
  if (task.status === 'doing') {
    return 'text-primary';
  }
  return 'text-muted-foreground/50 transition-colors hover:text-foreground';
}

function toggleLabel(task: Task): string {
  return task.status === 'done' ? '标记为待办' : '标记为已完成';
}

function dueClass(task: Task): string {
  if (task.status !== 'done' && isOverdue(task.due_date)) {
    return 'font-medium text-destructive';
  }
  if (task.status !== 'done' && isDueToday(task.due_date)) {
    return 'font-medium text-amber-500';
  }
  return '';
}
</script>

<template>
  <ul class="space-y-2">
    <li
      v-for="task of sorted"
      :key="task.id"
      class="group flex cursor-pointer items-center gap-3 rounded-lg border bg-card px-4 py-3 transition-colors hover:border-ring/40 hover:bg-muted/30"
      @click="emit('edit', task)"
    >
      <button
        type="button"
        class="shrink-0"
        :aria-label="toggleLabel(task)"
        :title="toggleLabel(task)"
        @click.stop="emit('toggle', task)"
      >
        <CircleCheck v-if="task.status === 'done'" class="size-5" :class="statusIconClass(task)" />
        <CircleDotDashed v-else-if="task.status === 'doing'" class="size-5" :class="statusIconClass(task)" />
        <Circle v-else class="size-5" :class="statusIconClass(task)" />
      </button>

      <div class="min-w-0 flex-1">
        <div class="flex min-w-0 items-center gap-2">
          <h3
            class="truncate text-sm font-medium"
            :class="task.status === 'done' ? 'text-muted-foreground line-through' : ''"
          >
            {{ task.title }}
          </h3>
          <FileText
            v-if="task.notes"
            class="size-3.5 shrink-0 text-muted-foreground"
            aria-label="有备注"
            :title="task.notes"
          />
        </div>
        <p v-if="task.notes" class="mt-0.5 line-clamp-1 text-xs text-muted-foreground">{{ task.notes }}</p>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <Badge v-if="task.priority !== 'none'" variant="secondary" class="gap-1.5">
          <span class="size-1.5 rounded-full" :class="priorityDotClass(task.priority)" aria-hidden="true" />
          {{ taskPriorityLabel(task.priority) }}
        </Badge>

        <span v-if="task.due_date" class="inline-flex items-center gap-1 text-xs tabular-nums" :class="dueClass(task)">
          <AlarmClock class="size-3.5" />
          {{ task.due_date }}
          <template v-if="task.status !== 'done' && isOverdue(task.due_date)">已逾期</template>
        </span>

        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              class="size-7 text-muted-foreground"
              aria-label="任务操作"
              @click.stop
            >
              <MoreHorizontal class="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" @click.stop>
            <DropdownMenuItem @click="emit('edit', task)">
              <Pencil class="size-3.5" />
              编辑
            </DropdownMenuItem>
            <DropdownMenuItem class="text-destructive focus:text-destructive" @click="emit('remove', task)">
              <Trash2 class="size-3.5" />
              删除
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  </ul>
</template>

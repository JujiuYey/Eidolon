<script setup lang="ts">
import { AlarmClock, Circle, CircleCheck, CircleDotDashed, Undo2 } from 'lucide-vue-next';

import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import type { Task } from '@/types/tasks';

import { priorityDotClass, taskPriorityLabel, todayDateString } from '../utils/display';
import { durationLabel } from '../utils/schedule';

/**
 * 日程视图左栏：快速添加 + 未排期任务清单。
 * 行可拖出（HTML5 DnD，拖到右侧网格落成时间块）；拖出高亮时作为「拖回取消排期」的落点。
 */

withDefaults(
  defineProps<{
    tasks: Task[];
    loading?: boolean;
    /** 网格里有块正被拖到本栏外沿（高亮提示松手即取消排期） */
    dropTargetActive?: boolean;
    quickInputPlaceholder?: string;
  }>(),
  { loading: false, dropTargetActive: false, quickInputPlaceholder: '记一件事，回车保存' },
);

const emit = defineEmits<{
  (e: 'submit', text: string): void;
  (e: 'toggle', task: Task): void;
  (e: 'click', task: Task): void;
  (e: 'drag-start', taskId: string): void;
  (e: 'drag-end'): void;
}>();

function submit(event: Event) {
  const target = event.target as HTMLInputElement;
  const text = target.value.trim();
  if (!text) {
    return;
  }
  emit('submit', text);
  target.value = '';
}

function statusIcon(task: Task): typeof Circle {
  if (task.status === 'done') {
    return CircleCheck;
  }
  if (task.status === 'doing') {
    return CircleDotDashed;
  }
  return Circle;
}
</script>

<template>
  <aside
    class="flex w-[300px] shrink-0 flex-col rounded-xl border bg-card transition-colors"
    :class="dropTargetActive ? 'border-primary/50 bg-primary/[0.04]' : ''"
    @dragover.prevent
    @drop.prevent
  >
    <div class="border-b border-border p-3">
      <Input :placeholder="quickInputPlaceholder" aria-label="快速添加任务" @keydown.enter="submit" />
    </div>

    <div class="flex items-center justify-between px-4 pb-1 pt-3">
      <h2 class="text-xs font-medium text-muted-foreground">未排期 · {{ tasks.length }}</h2>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
      <div v-if="loading" class="space-y-2 px-2 pt-2" aria-busy="true">
        <Skeleton v-for="index in 3" :key="index" class="h-12 w-full rounded-lg" />
      </div>

      <p v-else-if="tasks.length === 0" class="px-3 py-10 text-center text-xs text-muted-foreground">
        暂无未排期任务，都排好了
      </p>

      <ul v-else class="space-y-1.5">
        <li
          v-for="task of tasks"
          :key="task.id"
          class="flex cursor-grab items-start gap-2.5 rounded-lg px-2.5 py-2 transition-colors hover:bg-muted/50 active:cursor-grabbing"
          draggable="true"
          :aria-label="`拖动 ${task.title} 到时间格安排日程`"
          @dragstart="emit('drag-start', task.id)"
          @dragend="emit('drag-end')"
          @click="emit('click', task)"
        >
          <button
            type="button"
            class="mt-0.5 shrink-0 text-muted-foreground/50 transition-colors hover:text-foreground"
            :aria-label="task.status === 'done' ? '标记为待办' : '标记为已完成'"
            @click.stop="emit('toggle', task)"
          >
            <component :is="statusIcon(task)" class="size-4" :class="task.status !== 'todo' ? 'text-primary' : ''" />
          </button>

          <div class="min-w-0 flex-1">
            <p
              class="truncate text-xs font-medium leading-5"
              :class="task.status === 'done' ? 'text-muted-foreground line-through' : ''"
            >
              {{ task.title }}
            </p>
            <div class="mt-0.5 flex items-center gap-2 text-[10px] text-muted-foreground">
              <span v-if="task.priority !== 'none'" class="inline-flex items-center gap-1">
                <span class="size-1.5 rounded-full" :class="priorityDotClass(task.priority)" aria-hidden="true" />
                {{ taskPriorityLabel(task.priority) }}
              </span>
              <span v-if="task.estimate_minutes > 0">{{ durationLabel(task.estimate_minutes) }}</span>
              <span
                v-if="task.due_date"
                class="inline-flex items-center gap-0.5 tabular-nums"
                :class="task.status !== 'done' && task.due_date < todayDateString() ? 'text-destructive' : ''"
              >
                <AlarmClock class="size-3" />
                {{ task.due_date }}
              </span>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <!-- 拖块回清单时的落点提示（真实状态，非装饰） -->
    <div
      v-if="dropTargetActive"
      class="flex shrink-0 items-center justify-center gap-1.5 border-t border-primary/40 bg-primary/[0.06] px-3 py-2.5 text-xs font-medium text-primary"
    >
      <Undo2 class="size-3.5" />
      松手取消排期，任务回到清单
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ChevronLeft, ChevronRight } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import SagTimeGrid from '@/components/sag/sag-time-grid/index.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useTasksStore } from '@/stores/tasks';
import type { Task, TaskPriority } from '@/types/tasks';
import { getErrorMessage } from '@/utils/helpers';

import { sortTasks, todayDateString } from '../utils/display';
import {
  buildDatetime,
  dateKey,
  formatMinutes,
  hasSchedule,
  parseDatetime,
  parseTimeToMinutes,
  snapMinutes,
  taskToBlock,
  weekDays,
  type TimeBlock,
} from '../utils/schedule';
import TaskBacklogPane from './task-backlog-pane.vue';

/**
 * 日程视图（两栏）：左未排期清单，右时间网格。
 * 排期相关的读写都在这里落地（saveTask 全量保存），任务编辑/完成切换交给页面级的弹窗与处理器。
 */

const emit = defineEmits<{
  (e: 'edit-task', task: Task): void;
}>();

const store = useTasksStore();

// ===== 周导航与视图模式 =====

const thisMonday = weekDays(new Date())[0]!;
const weekOffset = ref(0);
const gridMode = ref<'week' | 'day'>('week');
const selectedDay = ref(todayDateString());

const todayKey = todayDateString();

/** 周视图从本周周一起；日视图跟随选中日所在周，导航按天走 */
const currentMonday = computed(() => {
  const monday = new Date(thisMonday);
  monday.setDate(monday.getDate() + weekOffset.value * 7);
  return monday;
});

const days = computed(() => {
  if (gridMode.value === 'week') {
    return weekDays(currentMonday.value).map(dateKey);
  }
  return [selectedDay.value];
});

const rangeLabel = computed(() => {
  const week = weekDays(currentMonday.value).map(dateKey);
  return `${week[0]!.slice(5).replace('-', '/')} – ${week[6]!.slice(5).replace('-', '/')}`;
});

function shift(deltaDays: number) {
  if (gridMode.value === 'week') {
    weekOffset.value += Math.sign(deltaDays);
    return;
  }
  const next = new Date(`${selectedDay.value}T00:00`);
  next.setDate(next.getDate() + deltaDays);
  selectedDay.value = dateKey(next);
  weekOffset.value = Math.floor(
    (new Date(`${selectedDay.value}T00:00`).getTime() - new Date(`${thisMonday}T00:00`).getTime()) /
      (7 * 24 * 60 * 60 * 1000),
  );
}

function goToday() {
  weekOffset.value = 0;
  selectedDay.value = todayKey;
}

// ===== now 线 =====

const nowMs = ref(Date.now());
let nowTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  nowTimer = setInterval(() => {
    nowMs.value = Date.now();
  }, 60_000);
});
onBeforeUnmount(() => {
  if (nowTimer) {
    clearInterval(nowTimer);
  }
});

// ===== 任务 → 块 / 清单 =====

const blocks = computed<TimeBlock[]>(() =>
  store.tasks.map(task => taskToBlock(task, todayKey)).filter((block): block is TimeBlock => block !== null),
);

const backlogTasks = computed(() =>
  sortTasks(store.tasks.filter(task => task.status !== 'done' && !hasSchedule(task))),
);

function taskById(id: string): Task | undefined {
  return store.tasks.find(task => task.id === id);
}

async function saveQuietly(task: Task, successMessage?: string) {
  try {
    await store.saveTask(task);
    if (successMessage) {
      toast.success(successMessage);
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '保存排期失败'));
  }
}

/** 默认块长：预估优先，无预估 30 分钟 */
function defaultDuration(task: Task): number {
  return task.estimate_minutes > 0 ? Math.min(task.estimate_minutes, 240) : 30;
}

// ===== 网格事件 =====

function onExternalDrop(date: string, start: number) {
  const task = draggingTaskId.value ? taskById(draggingTaskId.value) : undefined;
  draggingTaskId.value = '';
  if (!task) {
    return;
  }
  const duration = defaultDuration(task);
  const end = Math.min(start + duration, 24 * 60);
  void saveQuietly({ ...task, start_at: buildDatetime(date, start), end_at: buildDatetime(date, end) });
}

function onMoveBlock(id: string, date: string, start: number, end: number) {
  const task = taskById(id);
  if (!task) {
    return;
  }
  void saveQuietly({ ...task, start_at: buildDatetime(date, start), end_at: buildDatetime(date, end) });
}

function onResizeBlock(id: string, start: number, end: number) {
  const task = taskById(id);
  if (!task || !task.start_at) {
    return;
  }
  const date = parseDatetime(task.start_at)?.date ?? '';
  void saveQuietly({ ...task, start_at: buildDatetime(date, start), end_at: buildDatetime(date, end) });
}

async function onToggleBlock(id: string) {
  const task = taskById(id);
  if (!task) {
    return;
  }
  try {
    await store.toggleDone(task);
  } catch (error) {
    toast.error(getErrorMessage(error, '更新任务失败'));
  }
}

// 块拖出网格：落在左栏 = 取消排期，其余 = 什么都不做（网格已回弹）
const backlogDragOver = ref(false);

function onBlockDropOutside(id: string, x: number, y: number) {
  backlogDragOver.value = false;
  const pane = backlogRef.value?.$el as HTMLElement | undefined;
  const task = taskById(id);
  if (!pane || !task) {
    return;
  }
  const rect = pane.getBoundingClientRect();
  const inside = x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  if (inside && hasSchedule(task)) {
    void saveQuietly({ ...task, start_at: '', end_at: '' });
  }
}

// ===== 快速新建（框选/单击空白） =====

interface PendingCreate {
  date: string;
  start: number;
  end: number;
  title: string;
  priority: TaskPriority;
  x: number;
  y: number;
}

const pendingCreate = ref<PendingCreate | null>(null);

function onCreateBlock(date: string, start: number, end: number, x: number, y: number) {
  pendingCreate.value = { date, start, end, title: '', priority: 'none', x, y };
}

async function confirmCreate() {
  const pending = pendingCreate.value;
  if (!pending || !pending.title.trim()) {
    return;
  }
  pendingCreate.value = null;
  const start = snapMinutes(pending.start);
  const end = Math.max(start + 15, snapMinutes(pending.end));
  void saveQuietly({
    id: '',
    title: pending.title.trim(),
    notes: '',
    status: 'todo',
    priority: pending.priority,
    due_date: '',
    start_at: buildDatetime(pending.date, start),
    end_at: buildDatetime(pending.date, end),
    estimate_minutes: 0,
    source: 'manual',
    sort: 0,
    completed_at: null,
    created_at: 0,
    updated_at: 0,
  });
}

// ===== 左栏 HTML5 拖拽 =====

const draggingTaskId = ref('');
const backlogRef = ref<InstanceType<typeof TaskBacklogPane> | null>(null);

const dragDuration = computed(() => {
  const task = draggingTaskId.value ? taskById(draggingTaskId.value) : undefined;
  return task ? defaultDuration(task) : 30;
});

function cancelCreate() {
  pendingCreate.value = null;
}

function popoverStyle(): Record<string, string> {
  const pending = pendingCreate.value;
  if (!pending) {
    return { display: 'none' };
  }
  const left = Math.min(Math.max(pending.x - 130, 16), window.innerWidth - 276);
  const top = Math.min(pending.y + 12, window.innerHeight - 220);
  return { left: `${left}px`, top: `${top}px` };
}

const PRIORITY_OPTIONS: Array<{ value: TaskPriority; label: string }> = [
  { value: 'none', label: '无优先级' },
  { value: 'high', label: 'P1' },
  { value: 'medium', label: 'P2' },
  { value: 'low', label: 'P3' },
];

function setCreatePriority(value: unknown) {
  if (pendingCreate.value) {
    pendingCreate.value.priority = value as TaskPriority;
  }
}

function setCreateTime(event: Event, bound: 'start' | 'end') {
  const target = event.target as HTMLInputElement;
  const minutes = parseTimeToMinutes(target.value);
  if (minutes === null || !pendingCreate.value) {
    return;
  }
  pendingCreate.value[bound] = minutes;
}

function onStartChange(event: Event) {
  setCreateTime(event, 'start');
}

function onEndChange(event: Event) {
  setCreateTime(event, 'end');
}

function timeInputValue(minutes: number): string {
  return formatMinutes(minutes);
}

function onEditBlock(id: string) {
  const task = taskById(id);
  if (task) {
    emit('edit-task', task);
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 gap-4">
    <TaskBacklogPane
      ref="backlogRef"
      :tasks="backlogTasks"
      :loading="store.tasksLoading && !store.tasksLoaded"
      :drop-target-active="backlogDragOver"
      @submit="text => store.createFromText(text).catch(error => toast.error(getErrorMessage(error, '添加任务失败')))"
      @toggle="task => onToggleBlock(task.id)"
      @click="task => emit('edit-task', task)"
      @drag-start="id => (draggingTaskId = id)"
      @drag-end="draggingTaskId = ''"
    />

    <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
      <!-- 工具行 -->
      <div class="flex shrink-0 items-center gap-2">
        <div class="flex items-center rounded-lg border">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-7 rounded-r-none"
            aria-label="上一段时间"
            @click="shift(-1)"
          >
            <ChevronLeft class="size-4" />
          </Button>
          <span class="min-w-24 px-2 text-center text-xs tabular-nums text-muted-foreground">
            {{ gridMode === 'week' ? rangeLabel : selectedDay.slice(5).replace('-', '/') }}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-7 rounded-l-none"
            aria-label="下一段时间"
            @click="shift(1)"
          >
            <ChevronRight class="size-4" />
          </Button>
        </div>
        <Button type="button" variant="outline" size="sm" @click="goToday"> 今天 </Button>

        <div class="ml-auto flex items-center rounded-lg border p-0.5">
          <Button
            v-for="mode of ['week', 'day'] as const"
            :key="mode"
            type="button"
            variant="ghost"
            size="sm"
            class="h-6 px-2.5 text-xs"
            :class="gridMode === mode ? 'bg-muted font-medium' : 'text-muted-foreground'"
            @click="gridMode = mode"
          >
            {{ mode === 'week' ? '周' : '日' }}
          </Button>
        </div>
      </div>

      <!-- 网格 -->
      <SagTimeGrid
        :days="days"
        :blocks="blocks"
        :today-key="todayKey"
        :now-ms="nowMs"
        :external-drag-active="draggingTaskId !== ''"
        :external-duration="dragDuration"
        @external-drop="onExternalDrop"
        @move-block="onMoveBlock"
        @resize-block="onResizeBlock"
        @create-block="onCreateBlock"
        @click-block="onEditBlock"
        @toggle-block="onToggleBlock"
        @block-drag-outside="id => (backlogDragOver = id !== null)"
        @block-drop-outside="onBlockDropOutside"
      />
    </div>

    <!-- 框选快速新建 -->
    <teleport to="body">
      <div v-if="pendingCreate" class="fixed inset-0 z-50" @pointerdown.self="cancelCreate">
        <div
          class="fixed w-64 rounded-xl border bg-popover p-3 text-popover-foreground shadow-md"
          :style="popoverStyle()"
        >
          <div class="space-y-1.5">
            <Label for="schedule-create-title">新建任务</Label>
            <Input
              id="schedule-create-title"
              v-model="pendingCreate.title"
              placeholder="要做什么事"
              autofocus
              @keydown.enter="confirmCreate"
              @keydown.escape.stop="cancelCreate"
            />
          </div>
          <div class="mt-3 grid grid-cols-2 gap-2">
            <div class="space-y-1">
              <Label for="schedule-create-start" class="text-[10px]">开始</Label>
              <Input
                id="schedule-create-start"
                type="time"
                step="900"
                :model-value="timeInputValue(pendingCreate.start)"
                @change="onStartChange"
              />
            </div>
            <div class="space-y-1">
              <Label for="schedule-create-end" class="text-[10px]">结束</Label>
              <Input
                id="schedule-create-end"
                type="time"
                step="900"
                :model-value="timeInputValue(pendingCreate.end)"
                @change="onEndChange"
              />
            </div>
          </div>
          <div class="mt-3 space-y-1">
            <Label class="text-[10px]">优先级</Label>
            <div class="flex gap-1">
              <Button
                v-for="option of PRIORITY_OPTIONS"
                :key="option.value"
                type="button"
                variant="outline"
                size="sm"
                class="h-6 flex-1 px-0 text-[10px]"
                :class="
                  pendingCreate.priority === option.value ? 'border-primary/50 bg-primary/[0.06] font-medium' : ''
                "
                @click="setCreatePriority(option.value)"
              >
                {{ option.label }}
              </Button>
            </div>
          </div>
          <div class="mt-3 flex justify-end gap-2">
            <Button type="button" variant="ghost" size="sm" @click="cancelCreate"> 取消 </Button>
            <Button type="button" size="sm" :disabled="!pendingCreate.title.trim()" @click="confirmCreate">
              保存
            </Button>
          </div>
        </div>
      </div>
    </teleport>
  </div>
</template>

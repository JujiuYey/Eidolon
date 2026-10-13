<script setup lang="ts">
import { ref, watch } from 'vue';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import type { Task, TaskPriority, TaskStatus } from '@/types/tasks';

import { buildDatetime, durationLabel, parseDatetime, parseTimeToMinutes } from '../utils/schedule';

const props = defineProps<{
  open: boolean;
  /** null 表示新建 */
  task: Task | null;
}>();

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void;
  (e: 'save', task: Task): void;
  (e: 'delete', task: Task): void;
}>();

const title = ref('');
const notes = ref('');
const status = ref<TaskStatus>('todo');
const priority = ref<TaskPriority>('none');
const dueDate = ref('');
const estimateMinutes = ref(0);
// 排期三件套：日期 + 开始时间 + 时长；三者齐全才算已排期
const scheduleDate = ref('');
const scheduleStart = ref('');
const scheduleDuration = ref(60);

const STATUS_OPTIONS: Array<{ value: TaskStatus; label: string }> = [
  { value: 'todo', label: '待办' },
  { value: 'doing', label: '进行中' },
  { value: 'done', label: '已完成' },
];

const PRIORITY_OPTIONS: Array<{ value: TaskPriority; label: string }> = [
  { value: 'none', label: '无优先级' },
  { value: 'high', label: 'P1 紧急' },
  { value: 'medium', label: 'P2' },
  { value: 'low', label: 'P3' },
];

const ESTIMATE_OPTIONS = [0, 15, 30, 45, 60, 90, 120, 180, 240];
const DURATION_OPTIONS = [15, 30, 45, 60, 90, 120, 180, 240];

function estimateLabel(minutes: number): string {
  return minutes === 0 ? '不限' : durationLabel(minutes);
}

watch(
  () => props.open,
  open => {
    if (!open) {
      return;
    }
    const task = props.task;
    title.value = task?.title ?? '';
    notes.value = task?.notes ?? '';
    status.value = task?.status ?? 'todo';
    priority.value = task?.priority ?? 'none';
    dueDate.value = task?.due_date ?? '';
    estimateMinutes.value = task?.estimate_minutes ?? 0;

    const start = task ? parseDatetime(task.start_at) : null;
    const end = task ? parseDatetime(task.end_at) : null;
    if (start && end && end.minutes > start.minutes) {
      scheduleDate.value = start.date;
      scheduleStart.value = `${String(Math.floor(start.minutes / 60)).padStart(2, '0')}:${String(start.minutes % 60).padStart(2, '0')}`;
      scheduleDuration.value = Math.min(240, Math.max(15, Math.round((end.minutes - start.minutes) / 15) * 15));
    } else {
      scheduleDate.value = '';
      scheduleStart.value = '';
      scheduleDuration.value = task && task.estimate_minutes > 0 ? Math.min(240, task.estimate_minutes) : 60;
    }
  },
);

const isEditing = () => props.task !== null;
const canSave = () => title.value.trim().length > 0;
const hasSchedule = () => scheduleDate.value !== '' && scheduleStart.value !== '';

function setStatus(value: unknown) {
  status.value = value as TaskStatus;
}

function setPriority(value: unknown) {
  priority.value = value as TaskPriority;
}

function setEstimate(value: unknown) {
  estimateMinutes.value = Number(value);
}

function setDuration(value: unknown) {
  scheduleDuration.value = Number(value);
}

function clearSchedule() {
  scheduleDate.value = '';
  scheduleStart.value = '';
}

function save() {
  if (!canSave()) {
    return;
  }
  let startAt = '';
  let endAt = '';
  const startMinutes = parseTimeToMinutes(scheduleStart.value);
  if (scheduleDate.value && startMinutes !== null) {
    const duration = Math.max(15, scheduleDuration.value);
    const endMinutes = Math.min(24 * 60, startMinutes + duration);
    startAt = buildDatetime(scheduleDate.value, startMinutes);
    endAt = buildDatetime(scheduleDate.value, endMinutes);
  }
  emit('save', {
    ...(props.task ?? {
      id: '',
      source: 'manual',
      sort: 0,
      completed_at: null,
      created_at: 0,
      updated_at: 0,
    }),
    title: title.value.trim(),
    notes: notes.value,
    status: status.value,
    priority: priority.value,
    due_date: dueDate.value,
    start_at: startAt,
    end_at: endAt,
    estimate_minutes: estimateMinutes.value,
  });
}

function remove() {
  if (props.task) {
    emit('delete', props.task);
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="value => emit('update:open', value)">
    <DialogContent class="sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>{{ isEditing() ? '编辑任务' : '新建任务' }}</DialogTitle>
        <DialogDescription>任务保存在本机，可随时修改状态、截止日期与排期。</DialogDescription>
      </DialogHeader>

      <div class="grid gap-4">
        <div class="space-y-1.5">
          <Label for="task-editor-title">标题</Label>
          <Input id="task-editor-title" v-model="title" placeholder="要做什么事" @keydown.enter="save" />
        </div>

        <div class="space-y-1.5">
          <Label for="task-editor-notes">备注</Label>
          <Textarea id="task-editor-notes" v-model="notes" rows="3" placeholder="补充信息（可选）" />
        </div>

        <div class="grid gap-4 sm:grid-cols-4">
          <div class="space-y-1.5">
            <Label>状态</Label>
            <Select :model-value="status" @update:model-value="setStatus">
              <SelectTrigger aria-label="状态">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="option of STATUS_OPTIONS" :key="option.value" :value="option.value">
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="space-y-1.5">
            <Label>优先级</Label>
            <Select :model-value="priority" @update:model-value="setPriority">
              <SelectTrigger aria-label="优先级">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="option of PRIORITY_OPTIONS" :key="option.value" :value="option.value">
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="space-y-1.5">
            <Label>预估时长</Label>
            <Select :model-value="String(estimateMinutes)" @update:model-value="setEstimate">
              <SelectTrigger aria-label="预估时长">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="minutes of ESTIMATE_OPTIONS" :key="minutes" :value="String(minutes)">
                  {{ estimateLabel(minutes) }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="space-y-1.5">
            <Label for="task-editor-due">截止日期</Label>
            <Input id="task-editor-due" v-model="dueDate" type="date" />
          </div>
        </div>

        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <Label>排期</Label>
            <Button
              v-if="hasSchedule()"
              type="button"
              variant="ghost"
              size="sm"
              class="h-6 px-2 text-xs text-muted-foreground"
              @click="clearSchedule"
            >
              清除排期
            </Button>
          </div>
          <div class="grid grid-cols-3 gap-2">
            <Input v-model="scheduleDate" type="date" aria-label="排期日期" />
            <Input v-model="scheduleStart" type="time" step="900" aria-label="开始时间" />
            <Select :model-value="String(scheduleDuration)" @update:model-value="setDuration">
              <SelectTrigger aria-label="时长">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="minutes of DURATION_OPTIONS" :key="minutes" :value="String(minutes)">
                  {{ durationLabel(minutes) }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <p class="text-[10px] text-muted-foreground">填了日期和开始时间，任务就会出现在日程视图的时间格里。</p>
        </div>
      </div>

      <DialogFooter class="mt-2 sm:justify-between">
        <Button
          v-if="isEditing()"
          type="button"
          variant="outline"
          class="text-destructive hover:bg-destructive/10 hover:text-destructive"
          @click="remove"
        >
          删除
        </Button>
        <div class="flex items-center gap-2" :class="isEditing() ? '' : 'sm:ml-auto'">
          <Button type="button" variant="outline" @click="emit('update:open', false)"> 取消 </Button>
          <Button type="button" :disabled="!canSave()" @click="save"> 保存 </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

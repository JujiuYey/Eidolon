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
  },
);

const isEditing = () => props.task !== null;
const canSave = () => title.value.trim().length > 0;

function setStatus(value: unknown) {
  status.value = value as TaskStatus;
}

function setPriority(value: unknown) {
  priority.value = value as TaskPriority;
}

function save() {
  if (!canSave()) {
    return;
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
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ isEditing() ? '编辑任务' : '新建任务' }}</DialogTitle>
        <DialogDescription>任务保存在本机，可随时修改状态与截止日期。</DialogDescription>
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

        <div class="grid gap-4 sm:grid-cols-3">
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
            <Label for="task-editor-due">截止日期</Label>
            <Input id="task-editor-due" v-model="dueDate" type="date" />
          </div>
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

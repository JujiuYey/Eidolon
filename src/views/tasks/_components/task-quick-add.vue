<script setup lang="ts">
import { Loader2, RotateCcw, Sparkles } from 'lucide-vue-next';
import { ref, watch } from 'vue';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { AiDraftCandidate } from '@/stores/tasks';
import type { TaskDraft, TaskPriority } from '@/types/tasks';

const props = defineProps<{
  draft: AiDraftCandidate | null;
}>();

const emit = defineEmits<{
  (e: 'submit', text: string): void;
  (e: 'parse', text: string): void;
  (e: 'apply', draft: TaskDraft): void;
  (e: 'discard'): void;
}>();

const input = ref('');

/** 草稿的可编辑副本：AI 解析成功后初始化，用户可改字段再保存 */
const editable = ref<TaskDraft>({ title: '', priority: 'none', due_date: '' });

watch(
  () => props.draft,
  draft => {
    if (draft?.status === 'success') {
      editable.value = { ...draft.draft };
    }
  },
  { immediate: true },
);

const PRIORITY_OPTIONS: Array<{ value: TaskPriority; label: string }> = [
  { value: 'none', label: '无优先级' },
  { value: 'high', label: 'P1 紧急' },
  { value: 'medium', label: 'P2' },
  { value: 'low', label: 'P3' },
];

function submit() {
  const text = input.value.trim();
  if (!text) {
    return;
  }
  emit('submit', text);
  input.value = '';
}

function parse() {
  const text = input.value.trim();
  if (!text) {
    return;
  }
  emit('parse', text);
  input.value = '';
}

function retryParse() {
  if (props.draft?.input) {
    emit('parse', props.draft.input);
  }
}

function apply() {
  if (!editable.value.title.trim()) {
    return;
  }
  emit('apply', { ...editable.value });
}

function setPriority(value: unknown) {
  editable.value.priority = value as TaskPriority;
}
</script>

<template>
  <div class="space-y-3">
    <div class="flex items-center gap-2">
      <Input
        v-model="input"
        class="h-11 rounded-xl"
        placeholder="记一件事，回车保存；点 ✦ 让 AI 帮你整理"
        aria-label="快速添加任务"
        @keydown.enter="submit"
      />
      <Button
        type="button"
        variant="outline"
        size="icon"
        class="h-11 w-11 shrink-0 rounded-xl"
        :disabled="!input.trim() || draft?.status === 'generating'"
        aria-label="用 AI 整理任务"
        title="用 AI 整理任务"
        @click="parse"
      >
        <Loader2 v-if="draft?.status === 'generating'" class="size-4 animate-spin" />
        <Sparkles v-else class="size-4" />
      </Button>
    </div>

    <div v-if="draft" class="rounded-xl border bg-muted/30 p-4">
      <!-- 生成中 -->
      <div v-if="draft.status === 'generating'" class="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在整理…
      </div>

      <!-- 解析失败 -->
      <template v-else-if="draft.status === 'error'">
        <p class="text-sm text-destructive">{{ draft.errorMessage }}</p>
        <div class="mt-3 flex items-center gap-2">
          <Button type="button" variant="outline" size="sm" @click="retryParse">
            <RotateCcw class="size-3.5" />
            重试
          </Button>
          <Button type="button" variant="ghost" size="sm" @click="emit('discard')"> 放弃 </Button>
        </div>
      </template>

      <!-- 草稿预览（可编辑） -->
      <template v-else>
        <div class="grid gap-3 sm:grid-cols-2">
          <div class="space-y-1.5 sm:col-span-2">
            <Label for="task-draft-title">标题</Label>
            <Input id="task-draft-title" v-model="editable.title" />
          </div>
          <div class="space-y-1.5">
            <Label>优先级</Label>
            <Select :model-value="editable.priority" @update:model-value="setPriority">
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
            <Label for="task-draft-due">截止日期</Label>
            <Input id="task-draft-due" v-model="editable.due_date" type="date" />
          </div>
        </div>
        <div class="mt-4 flex items-center justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" @click="emit('discard')"> 放弃 </Button>
          <Button type="button" size="sm" :disabled="!editable.title.trim()" @click="apply"> 保存任务 </Button>
        </div>
      </template>
    </div>
  </div>
</template>

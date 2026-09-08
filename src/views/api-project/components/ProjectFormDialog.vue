<script setup lang="ts">
import { FolderPlus, Pencil } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
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

export type ProjectFormMode = 'create' | 'edit';

interface Props {
  open: boolean;
  mode: ProjectFormMode;
  initialName?: string;
  initialDescription?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'submit', payload: { name: string; description: string }): void;
  (e: 'cancel'): void;
}>();

const name = ref('');
const description = ref('');
const nameInputRef = ref<HTMLInputElement | null>(null);
const submitting = ref(false);

const trimmedName = computed(() => name.value.trim());
const trimmedDescription = computed(() => description.value.trim());
const canSubmit = computed(() => trimmedName.value.length > 0 && !submitting.value);

const titleText = computed(() => (props.mode === 'create' ? '新建项目' : '编辑项目'));
const submitText = computed(() => (props.mode === 'create' ? '创建并进入' : '保存'));
const descriptionText = computed(() => (props.mode === 'create'
  ? '创建项目后会自动生成「默认分组」，用于管理接口请求。'
  : '修改项目的名称和描述。'));

function applyInitial(): void {
  name.value = props.initialName ?? '';
  description.value = props.initialDescription ?? '';
}

function reset(): void {
  name.value = '';
  description.value = '';
}

watch(
  () => props.open,
  open => {
    if (open) {
      submitting.value = false;
      applyInitial();
      setTimeout(() => nameInputRef.value?.focus(), 0);
    }
  },
);

watch(
  () => [props.initialName, props.initialDescription, props.mode, props.open] as const,
  () => {
    if (props.open) {
      applyInitial();
    }
  },
);

function onOpenChange(open: boolean): void {
  emit('update:open', open);
  if (!open) {
    emit('cancel');
    reset();
  }
}

function onSubmit(): void {
  if (!canSubmit.value) {
    return;
  }
  submitting.value = true;
  emit('submit', {
    name: trimmedName.value,
    description: trimmedDescription.value,
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <div class="flex items-center gap-2">
          <FolderPlus v-if="mode === 'create'" class="size-5 text-primary" />
          <Pencil v-else class="size-5 text-primary" />
          <DialogTitle>{{ titleText }}</DialogTitle>
        </div>
        <DialogDescription>{{ descriptionText }}</DialogDescription>
      </DialogHeader>

      <form class="flex flex-col gap-4" @submit.prevent="onSubmit">
        <div class="flex flex-col gap-1">
          <Label for="project-form-name">项目名称</Label>
          <Input
            id="project-form-name"
            ref="nameInputRef"
            v-model="name"
            placeholder="例如：病案质控管理系统"
            :disabled="submitting"
            required
          />
        </div>

        <div class="flex flex-col gap-1">
          <Label for="project-form-description">项目描述</Label>
          <Input
            id="project-form-description"
            v-model="description"
            placeholder="可选：用途、负责人、基地址说明"
            :disabled="submitting"
          />
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            :disabled="submitting"
            @click="onOpenChange(false)"
          >
            取消
          </Button>
          <Button type="submit" :disabled="!canSubmit">
            {{ submitText }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

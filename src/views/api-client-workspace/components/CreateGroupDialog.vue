<script setup lang="ts">
import { FolderPlus } from 'lucide-vue-next';
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

interface Props {
  open: boolean;
  projectName?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'submit', payload: { name: string }): void;
  (e: 'cancel'): void;
}>();

const name = ref('');
const submitting = ref(false);
const nameInputRef = ref<HTMLInputElement | null>(null);

const trimmedName = computed(() => name.value.trim());
const canSubmit = computed(() => trimmedName.value.length > 0 && !submitting.value);

function reset(): void {
  name.value = '';
}

watch(
  () => props.open,
  open => {
    if (open) {
      submitting.value = false;
      setTimeout(() => nameInputRef.value?.focus(), 0);
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
  emit('submit', { name: trimmedName.value });
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <div class="flex items-center gap-2">
          <FolderPlus class="size-5 text-primary" />
          <DialogTitle>新建分组</DialogTitle>
        </div>
        <DialogDescription>
          <span v-if="projectName">将在项目「{{ projectName }}」下创建分组。</span>
          <span v-else>将创建一个新分组。</span>
        </DialogDescription>
      </DialogHeader>

      <form class="flex flex-col gap-4" @submit.prevent="onSubmit">
        <div class="flex flex-col gap-1">
          <Label for="create-group-name">分组名称</Label>
          <Input
            id="create-group-name"
            ref="nameInputRef"
            v-model="name"
            placeholder="例如：订单管理"
            :disabled="submitting"
            required
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
            创建
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

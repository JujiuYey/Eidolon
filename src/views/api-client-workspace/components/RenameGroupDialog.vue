<script setup lang="ts">
import { Pencil } from 'lucide-vue-next';
import { computed, ref } from 'vue';
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
  currentName?: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'submit', payload: { name: string }): void;
  (e: 'cancel'): void;
}>();

const name = ref('');
const submitting = ref(false);

const trimmedName = computed(() => name.value.trim());
const originalName = computed(() => (props.currentName ?? '').trim());
const canSubmit = computed(() =>
  trimmedName.value.length > 0
  && trimmedName.value !== originalName.value
  && !submitting.value,
);

function reset(): void {
  name.value = '';
}

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
          <Pencil class="size-5 text-primary" />
          <DialogTitle>重命名分组</DialogTitle>
        </div>
        <DialogDescription>
          修改分组名称；保存后立即生效。
        </DialogDescription>
      </DialogHeader>

      <form class="flex flex-col gap-4" @submit.prevent="onSubmit">
        <div class="flex flex-col gap-1">
          <Label for="rename-group-name" class="mb-1">分组名称</Label>
          <Input
            id="rename-group-name"
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
            保存
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next';
import { computed } from 'vue';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface Props {
  open: boolean;
  title: string;
  description?: string;
  /** 危险态按钮；也可用 type="destructive" 表达同一件事 */
  type?: 'default' | 'destructive';
  confirmLabel?: string;
  cancelLabel?: string;
}

const props = withDefaults(defineProps<Props>(), {
  description: '',
  type: 'default',
  confirmLabel: '确认',
  cancelLabel: '取消',
});

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'confirm'): void;
  /** 关闭时触发（含点取消、点遮罩、按 ESC），调用方据此回落到「取消」分支 */
  (e: 'cancel'): void;
}>();

const destructive = computed(() => props.type === 'destructive');

/** 点取消、点遮罩、按 ESC 都走这里：关闭的同时抛出 cancel */
function close(): void {
  emit('update:open', false);
  emit('cancel');
}

function confirm(): void {
  emit('confirm');
  emit('update:open', false);
}

/**
 * Dialog 自身关闭（ESC / 点遮罩）时不会经过上面的按钮，
 * 必须在这里补一次 cancel——否则 useConfirm() 里 await 的 Promise 永远不 settle。
 */
function onOpenChange(open: boolean): void {
  emit('update:open', open);
  if (!open) {
    emit('cancel');
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <div class="flex items-start gap-3">
          <AlertTriangle v-if="destructive" class="mt-0.5 size-5 shrink-0 text-destructive" />
          <div class="flex-1">
            <DialogTitle>{{ title }}</DialogTitle>
            <DialogDescription v-if="description">{{ description }}</DialogDescription>
          </div>
        </div>
      </DialogHeader>
      <DialogFooter>
        <Button variant="outline" @click="close">
          {{ cancelLabel }}
        </Button>
        <Button :variant="type" @click="confirm">
          {{ confirmLabel }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

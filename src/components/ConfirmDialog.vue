<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next';
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';

interface ConfirmDialogProps {
  title?: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

interface Props extends ConfirmDialogProps {
  open: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'confirm'): void;
  (e: 'cancel'): void;
}>();

const dialogTitle = computed(() => props.title || '请确认');
const dialogDescription = computed(() => props.description || '');
const confirmLabel = computed(() => props.confirmLabel || '确认');
const cancelLabel = computed(() => props.cancelLabel || '取消');
const destructive = computed(() => props.destructive ?? false);

function handleOpenChange(open: boolean): void {
  emit('update:open', open);
  if (!open) {
    emit('cancel');
  }
}

function handleConfirm(): void {
  emit('confirm');
  emit('update:open', false);
}

function handleCancel(): void {
  emit('cancel');
  emit('update:open', false);
}
</script>

<template>
  <Dialog :open="open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <div class="flex items-start gap-3">
          <AlertTriangle v-if="destructive" class="mt-0.5 size-5 text-destructive" />
          <div class="flex-1">
            <DialogTitle>{{ dialogTitle }}</DialogTitle>
            <DialogDescription>{{ dialogDescription }}</DialogDescription>
          </div>
        </div>
      </DialogHeader>
      <DialogFooter>
        <Button variant="outline" @click="handleCancel">
          {{ cancelLabel }}
        </Button>
        <Button :variant="destructive ? 'destructive' : 'default'" @click="handleConfirm">
          {{ confirmLabel }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

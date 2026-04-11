<script setup lang="ts">
interface Props {
  open: boolean;
  title: string;
  description: string;
  type: 'default' | 'destructive';
}

interface Emits {
  (e: 'update:open', value: boolean): void;
  (e: 'confirm'): void;
}

defineProps<Props>();
defineEmits<Emits>();
</script>

<template>
  <Dialog :open="open" @update:open="$emit('update:open', $event)">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription>
          {{ description }}
        </DialogDescription>
      </DialogHeader>
      <div class="flex justify-end gap-2 pt-4">
        <Button variant="outline" @click="$emit('update:open', false)">
          取消
        </Button>
        <Button :variant="type" @click="$emit('confirm')">
          确认
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>

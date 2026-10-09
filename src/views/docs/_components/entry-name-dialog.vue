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

interface Props {
  open: boolean;
  title: string;
  description: string;
  initialName?: string;
  confirmLabel?: string;
}

interface Emits {
  (e: 'update:open', value: boolean): void;
  (e: 'confirm', name: string): void;
}

const props = withDefaults(defineProps<Props>(), {
  initialName: '',
  confirmLabel: '创建',
});

const emit = defineEmits<Emits>();

const draft = ref('');

watch(
  () => props.open,
  open => {
    if (open) {
      draft.value = props.initialName;
    }
  },
);

function confirm() {
  const name = draft.value.trim();
  if (name) {
    emit('confirm', name);
  }
}
</script>

<template>
  <Dialog
    :open="open"
    @update:open="value => emit('update:open', value)"
  >
    <DialogContent class="sm:max-w-sm">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription>{{ description }}</DialogDescription>
      </DialogHeader>
      <Input
        v-model="draft"
        aria-label="名称"
        maxlength="80"
        @keydown.enter="confirm"
      />
      <DialogFooter>
        <Button
          variant="outline"
          @click="emit('update:open', false)"
        >
          取消
        </Button>
        <Button
          :disabled="!draft.trim()"
          @click="confirm"
        >
          {{ confirmLabel }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { Plus, Trash2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import type { ApiClientKeyValueRow } from '@/types/api-client';

interface Props {
  modelValue: ApiClientKeyValueRow[];
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  addLabel?: string;
  emptyHint?: string;
  showToggle?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  keyPlaceholder: '名称',
  valuePlaceholder: '值',
  addLabel: '新增一行',
  emptyHint: '暂无内容',
  showToggle: true,
});

const emit = defineEmits<{
  'update:modelValue': [rows: ApiClientKeyValueRow[]];
}>();

function updateRow(rowId: string, patch: Partial<ApiClientKeyValueRow>): void {
  emit(
    'update:modelValue',
    props.modelValue.map(row => (row.id === rowId ? { ...row, ...patch } : row)),
  );
}

function removeRow(rowId: string): void {
  emit(
    'update:modelValue',
    props.modelValue.filter(row => row.id !== rowId),
  );
}

function addRow(): void {
  emit('update:modelValue', [
    ...props.modelValue,
    {
      id: `row_${Math.random().toString(36).slice(2, 10)}`,
      enabled: true,
      key: '',
      value: '',
    },
  ]);
}

function onKeyInput(rowId: string, value: string | number): void {
  updateRow(rowId, { key: String(value ?? '') });
}

function onValueInput(rowId: string, value: string | number): void {
  updateRow(rowId, { value: String(value ?? '') });
}

function onToggle(rowId: string, value: boolean | string | number): void {
  updateRow(rowId, { enabled: Boolean(value) });
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <div
      v-if="modelValue.length === 0"
      class="rounded-md border border-dashed bg-muted/20 px-3 py-4 text-center text-xs text-muted-foreground"
    >
      {{ emptyHint }}
    </div>

    <div
      v-for="row of modelValue"
      v-else
      :key="row.id"
      class="grid grid-cols-[auto_1fr_1fr_auto] items-center gap-2"
    >
      <Switch
        v-if="showToggle"
        :model-value="row.enabled"
        @update:model-value="value => onToggle(row.id, value)"
      />
      <span v-else class="size-9" />

      <Input
        :model-value="row.key"
        :placeholder="keyPlaceholder"
        :disabled="showToggle && !row.enabled"
        @update:model-value="value => onKeyInput(row.id, value)"
      />

      <Input
        :model-value="row.value"
        :placeholder="valuePlaceholder"
        :disabled="showToggle && !row.enabled"
        @update:model-value="value => onValueInput(row.id, value)"
      />

      <Button
        variant="ghost"
        size="icon-sm"
        :aria-label="`删除行 ${row.key}`"
        @click="removeRow(row.id)"
      >
        <Trash2 class="size-4" />
      </Button>
    </div>

    <div class="flex justify-start">
      <Button variant="outline" size="sm" @click="addRow">
        <Plus class="size-4" />
        {{ addLabel }}
      </Button>
    </div>
  </div>
</template>

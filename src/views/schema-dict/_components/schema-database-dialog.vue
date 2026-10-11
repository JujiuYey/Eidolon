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
import type { SchemaDatabase, SchemaDbType } from '@/types/schema-dict';

import { SCHEMA_DB_TYPE_LABELS } from '../utils/query-templates';

const props = defineProps<{
  open: boolean;
  /** null 表示新建 */
  database: SchemaDatabase | null;
}>();

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void;
  (e: 'save', item: SchemaDatabase): void;
  (e: 'delete', item: SchemaDatabase): void;
}>();

const name = ref('');
const dbType = ref<SchemaDbType>('mysql');
const note = ref('');

watch(
  () => props.open,
  open => {
    if (!open) {
      return;
    }
    name.value = props.database?.name ?? '';
    dbType.value = props.database?.db_type ?? 'mysql';
    note.value = props.database?.note ?? '';
  },
);

const isEditing = () => props.database !== null;
const canSave = () => name.value.trim().length > 0;

function setDbType(value: unknown) {
  dbType.value = value as SchemaDbType;
}

function save() {
  if (!canSave()) {
    return;
  }
  emit('save', {
    ...(props.database ?? {
      id: '',
      sort: 0,
      created_at: 0,
      updated_at: 0,
    }),
    name: name.value.trim(),
    db_type: dbType.value,
    note: note.value,
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="value => emit('update:open', value)">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ isEditing() ? '编辑库' : '新建库' }}</DialogTitle>
        <DialogDescription>方言决定 DDL 解析规则和智能查询模板，导入后不可从快照反推。</DialogDescription>
      </DialogHeader>

      <div class="grid gap-4">
        <div class="space-y-1.5">
          <Label for="schema-db-name">库名</Label>
          <Input id="schema-db-name" v-model="name" placeholder="如：病案质控库" @keydown.enter="save" />
        </div>

        <div class="space-y-1.5">
          <Label>方言</Label>
          <Select :model-value="dbType" @update:model-value="setDbType">
            <SelectTrigger aria-label="方言">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="(labelText, type) of SCHEMA_DB_TYPE_LABELS" :key="type" :value="type">
                {{ labelText }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div class="space-y-1.5">
          <Label for="schema-db-note">备注</Label>
          <Textarea id="schema-db-note" v-model="note" rows="2" placeholder="连接方式、来源说明等（可选）" />
        </div>
      </div>

      <DialogFooter class="mt-2 sm:justify-between">
        <Button
          v-if="isEditing()"
          type="button"
          variant="outline"
          class="text-destructive hover:bg-destructive/10 hover:text-destructive"
          @click="emit('delete', props.database!)"
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

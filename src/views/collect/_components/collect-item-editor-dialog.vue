<script setup lang="ts">
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
import { Textarea } from '@/components/ui/textarea';
import type { CollectItem } from '@/types/collect';

const props = defineProps<{
  open: boolean;
  item: CollectItem | null;
}>();

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void;
  (e: 'save', item: CollectItem): void;
}>();

const title = ref('');
const content = ref('');

const isLink = computed(() => props.item?.kind === 'link');

watch(
  () => props.open,
  open => {
    if (!open) {
      return;
    }
    title.value = props.item?.title ?? '';
    content.value = props.item?.content ?? '';
  },
);

const canSave = () => (isLink.value ? true : content.value.trim().length > 0);

function save() {
  const item = props.item;
  if (!item || !canSave()) {
    return;
  }
  emit('save', {
    ...item,
    // 链接标题留空时清回 null，卡片继续用域名占位
    title: isLink.value ? title.value.trim() || null : item.title,
    content: content.value,
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="value => emit('update:open', value)">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ isLink ? '编辑链接' : '编辑记事' }}</DialogTitle>
        <DialogDescription v-if="isLink">{{ props.item?.url }}</DialogDescription>
      </DialogHeader>

      <div class="grid gap-4">
        <div v-if="isLink" class="space-y-1.5">
          <Label for="collect-editor-title">标题</Label>
          <Input id="collect-editor-title" v-model="title" placeholder="留空则显示域名" @keydown.enter="save" />
        </div>

        <div class="space-y-1.5">
          <Label for="collect-editor-content">{{ isLink ? '备注' : '正文' }}</Label>
          <Textarea
            id="collect-editor-content"
            v-model="content"
            :rows="isLink ? 3 : 6"
            :placeholder="isLink ? '补充信息（可选）' : '记点什么'"
          />
        </div>
      </div>

      <DialogFooter class="mt-2">
        <Button type="button" variant="outline" @click="emit('update:open', false)"> 取消 </Button>
        <Button type="button" :disabled="!canSave()" @click="save"> 保存 </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

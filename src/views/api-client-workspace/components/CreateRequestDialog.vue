<script setup lang="ts">
import { Plus } from 'lucide-vue-next';
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
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { API_CLIENT_HTTP_METHODS } from '@/types/api-client';
import type { ApiClientHttpMethod } from '@/types/api-client';

interface Props {
  open: boolean;
  groupName?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'submit', payload: { name: string; method: ApiClientHttpMethod; url: string }): void;
  (e: 'cancel'): void;
}>();

const name = ref('');
const method = ref<ApiClientHttpMethod>('GET');
const url = ref('');
const submitting = ref(false);
const nameInputRef = ref<HTMLInputElement | null>(null);

const trimmedName = computed(() => name.value.trim());
const canSubmit = computed(() => trimmedName.value.length > 0 && !submitting.value);

function reset(): void {
  name.value = '';
  method.value = 'GET';
  url.value = '';
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
  emit('submit', {
    name: trimmedName.value,
    method: method.value,
    url: url.value.trim(),
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <div class="flex items-center gap-2">
          <Plus class="size-5 text-primary" />
          <DialogTitle>新建请求</DialogTitle>
        </div>
        <DialogDescription>
          <span v-if="groupName">将在分组「{{ groupName }}」下创建请求。</span>
          <span v-else>将创建一个新请求。</span>
        </DialogDescription>
      </DialogHeader>

      <form class="flex flex-col gap-4" @submit.prevent="onSubmit">
        <div class="flex flex-col gap-1">
          <Label for="create-request-name">请求名称</Label>
          <Input
            id="create-request-name"
            ref="nameInputRef"
            v-model="name"
            placeholder="例如：新增订单"
            :disabled="submitting"
            required
          />
        </div>

        <div class="grid grid-cols-[120px_1fr] gap-3">
          <div class="flex flex-col gap-1">
            <Label for="create-request-method">方法</Label>
            <NativeSelect
              id="create-request-method"
              v-model="method"
              :disabled="submitting"
            >
              <NativeSelectOption v-for="m of API_CLIENT_HTTP_METHODS" :key="m" :value="m">
                {{ m }}
              </NativeSelectOption>
            </NativeSelect>
          </div>

          <div class="flex flex-col gap-1">
            <Label for="create-request-url">URL</Label>
            <Input
              id="create-request-url"
              v-model="url"
              placeholder="https://example.com/path"
              :disabled="submitting"
            />
          </div>
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

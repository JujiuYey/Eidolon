<script setup lang="ts">
import { Inbox, Link2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

import { isLinkText } from '../utils/display';

const emit = defineEmits<{
  (e: 'submit', text: string): void;
}>();

const input = ref('');
// 内容像链接时换图标，给「会被当链接抓取」一个轻提示
const looksLikeLink = computed(() => isLinkText(input.value));
const canSubmit = computed(() => input.value.trim().length > 0);

function submit() {
  const text = input.value.trim();
  if (!text) {
    return;
  }
  emit('submit', text);
  input.value = '';
}
</script>

<template>
  <div
    class="flex items-center gap-2 rounded-xl border border-primary/20 bg-primary/5 p-2 transition-colors focus-within:border-primary/40"
  >
    <component
      :is="looksLikeLink ? Link2 : Inbox"
      class="ml-1.5 size-4 shrink-0"
      :class="looksLikeLink ? 'text-primary' : 'text-muted-foreground'"
    />
    <Input
      v-model="input"
      class="border-0 px-2 shadow-none focus-visible:ring-0"
      placeholder="粘贴链接或随手记点什么"
      aria-label="快速收藏"
      @keydown.enter="submit"
    />
    <Button size="sm" :disabled="!canSubmit" @click="submit">
      收藏<span class="ml-1.5 text-xs opacity-70">↵</span>
    </Button>
  </div>
</template>

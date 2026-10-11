<script setup lang="ts">
import { ref } from 'vue';

import { Input } from '@/components/ui/input';

const emit = defineEmits<{
  (e: 'submit', text: string): void;
}>();

const input = ref('');

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
  <Input
    v-model="input"
    class="h-11 rounded-xl"
    placeholder="粘贴链接或随手记点什么，回车收藏"
    aria-label="快速收藏"
    @keydown.enter="submit"
  />
</template>

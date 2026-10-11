<script setup lang="ts">
import { Loader2, Send, Sparkles, X } from 'lucide-vue-next';
import { ref } from 'vue';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { AiAnswerState } from '@/stores/schema-dict';

defineProps<{
  answer: AiAnswerState | null;
  disabled: boolean;
}>();

const emit = defineEmits<{
  (e: 'ask', question: string): void;
  (e: 'clear'): void;
}>();

const question = ref('');

const SUGGESTIONS = ['这个库主要存什么业务数据？', '哪张表有身份证或手机号字段？', '列出所有主键不是自增 id 的表'];

function ask() {
  const text = question.value.trim();
  if (!text) {
    return;
  }
  emit('ask', text);
  question.value = '';
}

function askSuggestion(text: string) {
  question.value = '';
  emit('ask', text);
}
</script>

<template>
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4">
    <!-- 回答 -->
    <div v-if="answer && answer.status !== 'idle'" class="rounded-xl border bg-card p-4" aria-live="polite">
      <div class="flex items-center gap-2">
        <Sparkles class="size-4 text-primary" />
        <h3 class="min-w-0 truncate text-sm font-semibold">{{ answer.question }}</h3>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="ml-auto size-7 text-muted-foreground"
          aria-label="清除回答"
          @click="emit('clear')"
        >
          <X class="size-4" />
        </Button>
      </div>

      <div v-if="answer.status === 'loading'" class="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在查表结构…
      </div>
      <p v-else-if="answer.status === 'error'" class="mt-3 text-sm text-destructive">{{ answer.errorMessage }}</p>
      <p v-else class="mt-3 whitespace-pre-wrap text-sm leading-6">{{ answer.text }}</p>
    </div>

    <!-- 建议问题 -->
    <div v-if="!answer || answer.status === 'idle' || answer.status === 'error'" class="flex flex-wrap gap-2">
      <Button
        v-for="suggestion of SUGGESTIONS"
        :key="suggestion"
        type="button"
        variant="outline"
        size="sm"
        :disabled="disabled || answer?.status === 'loading'"
        @click="askSuggestion(suggestion)"
      >
        {{ suggestion }}
      </Button>
    </div>

    <!-- 提问输入 -->
    <div class="mt-auto flex items-center gap-2">
      <Input
        v-model="question"
        class="h-11 rounded-xl"
        placeholder="问点什么，比如：住院记录存在哪张表？"
        aria-label="表结构问题"
        :disabled="disabled"
        @keydown.enter="ask"
      />
      <Button
        type="button"
        size="icon"
        class="h-11 w-11 shrink-0 rounded-xl"
        :disabled="disabled || !question.trim() || answer?.status === 'loading'"
        aria-label="提问"
        @click="ask"
      >
        <Loader2 v-if="answer?.status === 'loading'" class="size-4 animate-spin" />
        <Send v-else class="size-4" />
      </Button>
    </div>
    <p v-if="disabled" class="text-center text-xs text-muted-foreground">需要先有一个库和至少一份快照</p>
  </div>
</template>

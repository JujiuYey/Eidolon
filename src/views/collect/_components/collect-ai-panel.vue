<script setup lang="ts">
import { Loader2, RotateCcw, Send, Sparkles, X } from 'lucide-vue-next';
import MarkdownRender from 'markstream-vue';
import { ref } from 'vue';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { AiAnswerState } from '@/stores/collect';

const props = defineProps<{
  answer: AiAnswerState;
  /** 收藏夹为空时禁止提问 */
  disabled: boolean;
}>();

const emit = defineEmits<{
  (e: 'ask', question: string): void;
  (e: 'clear'): void;
  (e: 'close'): void;
}>();

const question = ref('');

const SUGGESTIONS = ['我最近收藏了哪些内容？', '帮我找出讲数据库的收藏', '回忆一下我记过的记事'];

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

function retry() {
  if (props.answer.question) {
    emit('ask', props.answer.question);
  }
}
</script>

<template>
  <div class="rounded-xl border bg-card p-4" aria-live="polite">
    <div class="flex items-center gap-2">
      <Sparkles class="size-4 text-primary" />
      <h3 class="text-sm font-semibold">问收藏</h3>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        class="ml-auto size-7 text-muted-foreground"
        aria-label="收起问收藏"
        @click="emit('close')"
      >
        <X class="size-4" />
      </Button>
    </div>

    <!-- 回答 -->
    <div v-if="answer.status !== 'idle'" class="mt-3 rounded-lg bg-muted/40 p-3">
      <div class="flex items-center gap-2">
        <span class="min-w-0 truncate text-xs font-medium text-muted-foreground">{{ answer.question }}</span>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="ml-auto size-6 shrink-0 text-muted-foreground"
          aria-label="清除回答"
          @click="emit('clear')"
        >
          <X class="size-3.5" />
        </Button>
      </div>

      <div v-if="answer.status === 'loading'" class="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 class="size-4 animate-spin" />
        正在翻你的收藏…
      </div>
      <template v-else-if="answer.status === 'error'">
        <p class="mt-2 text-sm text-destructive">{{ answer.errorMessage }}</p>
        <Button type="button" variant="outline" size="sm" class="mt-2" @click="retry">
          <RotateCcw class="size-3.5" />
          重试
        </Button>
      </template>
      <div v-else class="collect-ai-answer mt-2 min-w-0 text-sm leading-6">
        <MarkdownRender :content="answer.text" custom-id="collect-ai-answer" />
      </div>
    </div>

    <!-- 建议问题 -->
    <div v-if="answer.status === 'idle' || answer.status === 'error'" class="mt-3 flex flex-wrap gap-2">
      <Button
        v-for="suggestion of SUGGESTIONS"
        :key="suggestion"
        type="button"
        variant="outline"
        size="sm"
        :disabled="disabled"
        @click="askSuggestion(suggestion)"
      >
        {{ suggestion }}
      </Button>
    </div>

    <!-- 提问输入 -->
    <div class="mt-3 flex items-center gap-2">
      <Input
        v-model="question"
        class="h-10 rounded-xl"
        placeholder="问点什么，比如：我收藏过讲正文提取的文章吗？"
        aria-label="收藏问题"
        :disabled="disabled"
        @keydown.enter="ask"
      />
      <Button
        type="button"
        size="icon"
        class="h-10 w-10 shrink-0 rounded-xl"
        :disabled="disabled || !question.trim() || answer.status === 'loading'"
        aria-label="提问"
        @click="ask"
      >
        <Loader2 v-if="answer.status === 'loading'" class="size-4 animate-spin" />
        <Send v-else class="size-4" />
      </Button>
    </div>
    <p v-if="disabled" class="mt-2 text-center text-xs text-muted-foreground">先收藏一点内容再来问吧</p>
  </div>
</template>

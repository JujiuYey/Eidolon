<script setup lang="ts">
import { Loader2, RotateCcw, X } from 'lucide-vue-next';

import { MessageResponse } from '@/components/ai-elements/message';
import SagAiPanel from '@/components/sag/sag-ai-panel/index.vue';
import { Button } from '@/components/ui/button';
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

const SUGGESTIONS = ['我最近收藏了哪些内容？', '帮我找出讲数据库的收藏', '回忆一下我记过的记事'];

function askSuggestion(text: string) {
  if (props.answer.status === 'loading') {
    return;
  }
  emit('ask', text);
}

function retry() {
  if (props.answer.question) {
    emit('ask', props.answer.question);
  }
}
</script>

<template>
  <SagAiPanel
    class="w-[21rem] shrink-0"
    title="问收藏"
    placeholder="问点什么，比如：我收藏过讲正文提取的文章吗？"
    :disabled="disabled"
    :busy="answer.status === 'loading'"
    :disabled-hint="disabled ? '先收藏一点内容再来问吧' : ''"
    @send="text => emit('ask', text)"
    @close="emit('close')"
  >
    <!-- 建议问题（未提问时） -->
    <div v-if="answer.status === 'idle'" class="space-y-2">
      <p class="text-xs text-muted-foreground">试着问问：</p>
      <button
        v-for="suggestion of SUGGESTIONS"
        :key="suggestion"
        type="button"
        class="w-full rounded-lg border bg-card px-3 py-2 text-left text-sm transition-colors hover:border-ring/40 hover:bg-muted/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50"
        :disabled="disabled"
        @click="askSuggestion(suggestion)"
      >
        {{ suggestion }}
      </button>
    </div>

    <!-- 回答 -->
    <div v-else class="rounded-lg bg-muted/40 p-3" aria-live="polite">
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
        <MessageResponse :content="answer.text" />
      </div>
    </div>
  </SagAiPanel>
</template>

<script setup lang="ts">
import { Loader2, Send, Sparkles, X } from 'lucide-vue-next';
import type { HTMLAttributes } from 'vue';
import { nextTick, ref } from 'vue';

import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

/**
 * AI 助手面板壳：统一的容器 / 头部 / 滚动体 / 高亮输入区。
 * 各功能（问收藏、文档 AI、字典问答…）只往默认插槽里放自己的内容；
 * 宽度由使用方通过 class 控制（如 asides 传 w-[21rem] shrink-0，页内 fluid 传 w-full）。
 */
const props = withDefaults(
  defineProps<{
    class?: HTMLAttributes['class'];
    /** 面板标题 */
    title: string;
    /** 输入框占位文案 */
    placeholder?: string;
    /** 禁用输入（如数据为空） */
    disabled?: boolean;
    /** 请求进行中：发送按钮转圈并禁用 */
    busy?: boolean;
    /** 显示右上角关闭按钮（页内常驻场景传 false） */
    closable?: boolean;
    /** disabled 时输入区下方的说明文字 */
    disabledHint?: string;
  }>(),
  { placeholder: '', disabled: false, busy: false, closable: true, disabledHint: '' },
);

const emit = defineEmits<{
  (e: 'send', text: string): void;
  (e: 'close'): void;
}>();

const question = ref('');

function send() {
  const text = question.value.trim();
  if (!text || props.disabled || props.busy) {
    return;
  }
  emit('send', text);
  question.value = '';
}

/** Enter 发送、Shift+Enter 换行；输入法选词时的 Enter 不当发送 */
function sendOnEnter(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) {
    return;
  }
  if (!event.shiftKey) {
    event.preventDefault();
    send();
  }
}

/** 供父组件回填输入框（如发送失败时把问题放回去） */
function setText(text: string) {
  question.value = text;
}

/** 滚动到内容底部（新消息到达时由父组件调用） */
const bodyRef = ref<HTMLElement | null>(null);

function scrollToBottom() {
  void nextTick(() => {
    const el = bodyRef.value;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  });
}

defineExpose({ setText, scrollToBottom });
</script>

<template>
  <aside :class="cn('flex h-full min-h-0 w-full flex-col overflow-hidden rounded-xl border bg-card', props.class)">
    <div class="flex h-11 shrink-0 items-center justify-between gap-2 border-b px-3">
      <p class="flex min-w-0 items-center gap-2 text-sm font-semibold">
        <Sparkles class="size-4 shrink-0 text-primary" />
        {{ title }}
      </p>
      <Button
        v-if="closable"
        type="button"
        variant="ghost"
        size="icon"
        class="size-7 shrink-0 text-muted-foreground"
        aria-label="关闭面板"
        @click="emit('close')"
      >
        <X class="size-4" />
      </Button>
    </div>

    <!-- 主体：唯一滚动区域 -->
    <div ref="bodyRef" class="min-h-0 flex-1 overflow-y-auto p-3">
      <slot />
    </div>

    <!-- 输入：高亮区块，聚焦时边框与底色加深 -->
    <div class="shrink-0 border-t p-3">
      <div
        class="rounded-xl border border-primary/30 bg-primary/5 p-2 transition-colors focus-within:border-primary/60 focus-within:bg-primary/10"
      >
        <Textarea
          v-model="question"
          class="max-h-40 min-h-14 resize-none border-0 px-2 py-1.5 shadow-none focus-visible:border-0 focus-visible:ring-0 dark:bg-transparent"
          :placeholder="placeholder"
          aria-label="提问输入"
          :disabled="disabled"
          @keydown.enter="sendOnEnter"
        />
        <div class="flex items-center justify-between gap-2 px-1 pt-1">
          <span class="text-xs text-muted-foreground">Enter 发送 · Shift+Enter 换行</span>
          <Button
            type="button"
            size="icon"
            class="size-8 shrink-0 rounded-lg"
            :disabled="disabled || !question.trim() || busy"
            aria-label="发送"
            @click="send"
          >
            <Loader2 v-if="busy" class="size-4 animate-spin" />
            <Send v-else class="size-4" />
          </Button>
        </div>
      </div>
      <p v-if="disabled && disabledHint" class="mt-2 text-center text-xs text-muted-foreground">{{ disabledHint }}</p>
    </div>
  </aside>
</template>

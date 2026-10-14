<script setup lang="ts">
import { Sparkles, X } from 'lucide-vue-next';
import type { HTMLAttributes } from 'vue';
import { nextTick, ref } from 'vue';

import { PromptInput, type PromptInputMessage } from '@/components/ai-elements/prompt-input';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

import PromptArea from './prompt-area.vue';

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

/**
 * 提交由 PromptInput 统一抛出：Enter 发送、Shift+Enter 换行、
 * 输入法选词保护都在它内部，这里只做禁用态拦截与空串过滤。
 */
function handleSubmit(payload: PromptInputMessage) {
  if (props.disabled || props.busy) {
    return;
  }
  const text = payload.text.trim();
  if (!text) {
    return;
  }
  emit('send', text);
}

const promptRef = ref<InstanceType<typeof PromptArea> | null>(null);

/** 供父组件回填输入框（如发送失败时把问题放回去） */
function setText(text: string) {
  promptRef.value?.setText(text);
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

    <!-- 输入：ai-elements 的 PromptInput，Enter/换行/输入法保护由其内部处理 -->
    <div class="shrink-0 p-3">
      <PromptInput @submit="handleSubmit">
        <PromptArea ref="promptRef" :busy="busy" :disabled="disabled" :placeholder="placeholder" />
      </PromptInput>
      <p v-if="disabled && disabledHint" class="mt-2 text-center text-xs text-muted-foreground">{{ disabledHint }}</p>
    </div>
  </aside>
</template>

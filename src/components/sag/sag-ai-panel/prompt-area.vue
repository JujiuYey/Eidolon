<script setup lang="ts">
import {
  PromptInputFooter,
  PromptInputSubmit,
  PromptInputTextarea,
  usePromptInput,
} from '@/components/ai-elements/prompt-input';

/**
 * PromptInput 的输入区。
 *
 * 单独拆一个组件是因为 usePromptInput() 走 inject——必须位于 PromptInput
 * 的子树内才能取到 context，父组件直接调用拿不到。通过 setText 把
 * 「发送失败后把问题放回输入框」这个能力透传给外层。
 *
 * 本组件刻意不产出任何包裹元素（多根 fragment）：PromptInput 内部的
 * InputGroup 靠 has-[>textarea]:h-auto 与 has-[>[data-align=block-end]]:flex-col
 * 两个选择器决定高度与纵向排布，一旦多一层 div 就全部失效，输入框会被压回
 * h-9 并溢出成错位的色块。
 *
 * 提交不在这里处理：PromptInputSubmit 是 type=submit，统一由 PromptInput
 * 的 @submit 抛出，Enter/点击/输入法保护都在 PromptInput 内部。
 */

interface Props {
  placeholder: string;
  disabled: boolean;
  busy: boolean;
}

const props = defineProps<Props>();

const input = usePromptInput();

function setText(text: string) {
  input.setTextInput(text);
}

defineExpose({ setText });
</script>

<template>
  <PromptInputTextarea
    class="resize-none border-0 bg-transparent shadow-none focus-visible:border-0 focus-visible:ring-0"
    :class="{ 'opacity-50': props.disabled }"
    :placeholder="placeholder"
    :disabled="disabled"
    aria-label="提问输入"
  />

  <PromptInputFooter class="justify-between gap-1">
    <span class="text-xs text-muted-foreground">Enter 发送 · Shift+Enter 换行</span>
    <PromptInputSubmit
      class="size-7 shrink-0 rounded-lg"
      :disabled="disabled || busy"
      :status="busy ? 'submitted' : 'ready'"
      aria-label="发送"
    />
  </PromptInputFooter>
</template>

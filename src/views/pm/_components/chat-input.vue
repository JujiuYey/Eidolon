<script setup lang="ts">
import type { ComponentPublicInstance } from 'vue';
import { SendHorizonal } from 'lucide-vue-next';
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import { Textarea } from '@/components/ui/textarea';

interface Props {
  sending: boolean;
  error: string;
}

interface Emits {
  (e: 'send', content: string): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const draft = ref('');
const inputElement = ref<ComponentPublicInstance | null>(null);

function focusTextarea() {
  const element = inputElement.value?.$el;
  if (element instanceof HTMLTextAreaElement) {
    element.focus();
  }
}

function submit() {
  const content = draft.value.trim();
  if (!content || props.sending) {
    return;
  }
  emit('send', content);
}

function handleKeydown(event: KeyboardEvent) {
  // Enter 发送；Shift+Enter 保留为换行
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    submit();
  }
}

/** 发送成功后由父组件调用，清空草稿并把焦点还给输入框 */
function clear() {
  draft.value = '';
  focusTextarea();
}

function focus() {
  focusTextarea();
}

defineExpose({ clear, focus });
</script>

<template>
  <div class="shrink-0 border-t px-6 py-4">
    <p
      v-if="error"
      class="mb-2 text-xs text-destructive"
    >
      {{ error }} 内容已保留，可直接重试。
    </p>

    <form
      class="mx-auto flex max-w-3xl items-end gap-2"
      @submit.prevent="submit"
    >
      <Textarea
        ref="inputElement"
        v-model="draft"
        aria-label="给产品经理发消息"
        class="max-h-44 min-h-11 resize-none"
        placeholder="说说你的产品问题（Enter 发送，Shift+Enter 换行）"
        :disabled="sending"
        @keydown="handleKeydown"
      />
      <Button
        aria-label="发送"
        class="h-11 shrink-0"
        type="submit"
        :disabled="sending || !draft.trim()"
      >
        <Spinner
          v-if="sending"
          class="h-4 w-4"
        />
        <SendHorizonal
          v-else
          class="h-4 w-4"
        />
        {{ sending ? '思考中' : '发送' }}
      </Button>
    </form>
  </div>
</template>

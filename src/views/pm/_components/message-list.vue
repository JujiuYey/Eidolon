<script setup lang="ts">
import type { PmMessage } from '@/types/pm';
import MarkdownRender from 'markstream-vue';
import { Bot } from 'lucide-vue-next';
import { nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Spinner } from '@/components/ui/spinner';

interface Props {
  messages: PmMessage[];
  isLoading: boolean;
  sending: boolean;
  hasConversation: boolean;
}

interface Emits {
  (e: 'send', content: string): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const EXAMPLE_PROMPTS = [
  '我想给内部工具加一个新功能，帮我梳理需求和边界',
  '这个注册流程的转化一直上不去，从哪里开始分析？',
  '帮我用可用性启发法过一遍现在的设计稿',
];

const scrollElement = ref<HTMLElement | null>(null);
const elapsedSeconds = ref(0);
let elapsedTimer: ReturnType<typeof setInterval> | null = null;

watch(
  () => [props.messages.length, props.sending] as const,
  async () => {
    await nextTick();
    const element = scrollElement.value;
    if (element) {
      element.scrollTop = element.scrollHeight;
    }
  },
);

watch(
  () => props.sending,
  sending => {
    if (sending && !elapsedTimer) {
      elapsedSeconds.value = 0;
      elapsedTimer = setInterval(() => {
        elapsedSeconds.value += 1;
      }, 1000);
    } else if (!sending && elapsedTimer) {
      clearInterval(elapsedTimer);
      elapsedTimer = null;
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (elapsedTimer) {
    clearInterval(elapsedTimer);
  }
});

const showEmptyState = () => !props.isLoading && props.messages.length === 0;
</script>

<template>
  <div
    ref="scrollElement"
    class="min-h-0 flex-1 overflow-y-auto px-6 py-6"
  >
    <template v-if="isLoading">
      <div class="space-y-6">
        <Skeleton class="ml-auto h-10 w-1/3 rounded-2xl" />
        <Skeleton class="h-16 w-2/3 rounded-2xl" />
        <Skeleton class="ml-auto h-10 w-1/4 rounded-2xl" />
      </div>
    </template>

    <div
      v-else-if="showEmptyState()"
      class="flex h-full flex-col items-center justify-center gap-4 py-10 text-center"
    >
      <div class="flex h-11 w-11 items-center justify-center rounded-full border bg-card">
        <Bot class="h-5 w-5 text-primary" />
      </div>
      <div>
        <p class="text-sm font-semibold">
          {{ hasConversation ? '新对话，直接开始提问' : '和你的产品经理聊聊' }}
        </p>
        <p class="mt-1 text-xs text-muted-foreground">
          产品、需求、设计问题都可以直接问，他会自动套用合适的方法论框架。
        </p>
      </div>
      <div class="flex max-w-lg flex-col gap-2">
        <button
          v-for="prompt of EXAMPLE_PROMPTS"
          :key="prompt"
          type="button"
          class="rounded-lg border bg-card px-4 py-2.5 text-left text-sm text-foreground transition-colors hover:bg-muted/60"
          @click="emit('send', prompt)"
        >
          {{ prompt }}
        </button>
      </div>
    </div>

    <div
      v-else
      class="mx-auto flex max-w-3xl flex-col gap-6"
    >
      <div
        v-for="message of messages"
        :key="message.id"
        class="flex gap-3"
        :class="message.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <!-- 用户消息：右侧气泡 -->
        <div
          v-if="message.role === 'user'"
          class="max-w-[75%] rounded-2xl bg-muted px-4 py-2.5 text-sm whitespace-pre-wrap break-words"
        >
          {{ message.content }}
        </div>

        <!-- 助手消息：头像 + Markdown -->
        <template v-else>
          <div class="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full border bg-card">
            <Bot class="h-4 w-4 text-primary" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="pm-markdown text-sm leading-relaxed">
              <MarkdownRender
                :content="message.content"
                custom-id="pm-message-list"
              />
            </div>
            <div
              v-if="message.skills_used.length > 0"
              class="mt-2 flex flex-wrap items-center gap-1.5"
            >
              <span class="text-xs text-muted-foreground">参考框架</span>
              <Badge
                v-for="slug of message.skills_used"
                :key="slug"
                variant="secondary"
                class="font-mono text-[11px]"
              >
                {{ slug }}
              </Badge>
            </div>
          </div>
        </template>
      </div>

      <!-- 发送中指示 -->
      <div
        v-if="sending"
        class="flex items-center gap-2 text-xs text-muted-foreground"
      >
        <Spinner class="h-3.5 w-3.5" />
        <span>正在思考</span>
        <span class="tabular-nums">{{ elapsedSeconds }}s</span>
      </div>
    </div>
  </div>
</template>

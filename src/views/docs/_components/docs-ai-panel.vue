<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { toast } from 'vue-sonner';

import { MessageResponse } from '@/components/ai-elements/message';
import SagAiPanel from '@/components/sag/sag-ai-panel/index.vue';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import type { DocsAiChatMessage } from '@/services/docs';
import { chatDocsAi } from '@/services/docs';
import { getErrorMessage } from '@/utils/helpers';

/**
 * 文档页左侧 AI 助手面板:带整篇文档上下文对话。
 * 模型按约定把"修改后的完整文档"放进 ````eidolon-doc 围栏,
 * 面板解析出围栏后提供 应用到文档 / 撤销 操作,实际写入由父组件完成。
 * 会话按文档分别保存在内存里：组件不再随文档切换重建，
 * 切走再切回来，之前那篇的对话还在（早先靠 :key 重建，切回即清空）。
 */
const props = defineProps<{
  activePath: string;
  document: string;
}>();

const emit = defineEmits<{
  (e: 'apply', content: string): void;
  (e: 'restore', content: string): void;
  (e: 'close'): void;
}>();

interface PanelMessage {
  id: number;
  role: 'user' | 'assistant';
  /** 展示用文本(assistant 已去掉 eidolon-doc 围栏) */
  text: string;
  /** 围栏里的新文档全文,普通回答为 null */
  doc: string | null;
  /** 是否已应用到文档 */
  applied: boolean;
  /** 应用那一刻的文档内容,撤销时恢复 */
  undoContent: string | null;
  modelLabel: string;
}

// 四反引号围栏优先(文档内部可能含普通代码块),三反引号兜底
const DOC_FENCE_RE = /(`{4,})eidolon-doc[^\n]*\n([\s\S]*?)\n[ \t]*\1/;
const DOC_FENCE3_RE = /```eidolon-doc[^\n]*\n([\s\S]*?)\n[ \t]*```/;

function parseAssistant(content: string): { text: string; doc: string | null } {
  const match = content.match(DOC_FENCE_RE);
  if (match && match.index !== undefined) {
    const text = (content.slice(0, match.index) + content.slice(match.index + match[0].length)).trim();
    return { text, doc: match[2] ?? null };
  }
  const fallback = content.match(DOC_FENCE3_RE);
  if (fallback && fallback.index !== undefined) {
    const text = (content.slice(0, fallback.index) + content.slice(fallback.index + fallback[0].length)).trim();
    return { text, doc: fallback[1] ?? null };
  }
  return { text: content, doc: null };
}

/** 历史消息里把围栏全文换成占位,防止上下文被撑爆 */
function stripDocForHistory(content: string): string {
  return content.replace(/(`{3,})eidolon-doc[^\n]*\n[\s\S]*?\n[ \t]*\1/g, '（本轮已生成文档新版本，全文略）');
}

/** 每篇文档一份会话；数组放在 reactive Map 里，push 就地改也能触发更新 */
const sessions = ref(new Map<string, PanelMessage[]>());
const messages = computed<PanelMessage[]>({
  get: () => sessions.value.get(props.activePath) ?? [],
  set: value => {
    sessions.value.set(props.activePath, value);
  },
});
const panelRef = ref<InstanceType<typeof SagAiPanel> | null>(null);
const sending = ref(false);
const elapsedSeconds = ref(0);
const expandedIds = ref<Set<number>>(new Set());

let messageSeq = 0;
let timer: ReturnType<typeof setInterval> | null = null;

const hasDocument = computed(() => props.activePath.length > 0);

function startTimer() {
  elapsedSeconds.value = 0;
  timer = setInterval(() => {
    elapsedSeconds.value += 1;
  }, 1000);
}

function stopTimer() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

function scrollToEnd() {
  panelRef.value?.scrollToBottom();
}

async function send(text: string) {
  if (!text || sending.value) {
    return;
  }

  const userMessage: PanelMessage = {
    id: ++messageSeq,
    role: 'user',
    text,
    doc: null,
    applied: false,
    undoContent: null,
    modelLabel: '',
  };
  messages.value.push(userMessage);
  scrollToEnd();

  sending.value = true;
  startTimer();
  try {
    const history: DocsAiChatMessage[] = messages.value.slice(0, -1).map(message => ({
      role: message.role,
      content: message.role === 'assistant' ? stripDocForHistory(message.text) : message.text,
    }));

    const result = await chatDocsAi(history, props.document, text);
    const parsed = parseAssistant(result.content);
    messages.value.push({
      id: ++messageSeq,
      role: 'assistant',
      text: parsed.text,
      doc: parsed.doc,
      applied: false,
      undoContent: null,
      modelLabel: result.model_label,
    });
  } catch (error) {
    // 失败:撤回这条用户消息,内容放回输入框
    messages.value = messages.value.filter(message => message.id !== userMessage.id);
    panelRef.value?.setText(text);
    toast.error(getErrorMessage(error, 'AI 请求失败'));
  } finally {
    stopTimer();
    sending.value = false;
    scrollToEnd();
  }
}

function applyDoc(message: PanelMessage) {
  if (message.doc === null) {
    return;
  }
  message.undoContent = props.document;
  message.applied = true;
  emit('apply', message.doc);
}

function undoApply(message: PanelMessage) {
  if (message.undoContent === null) {
    return;
  }
  message.applied = false;
  const restore = message.undoContent;
  message.undoContent = null;
  emit('restore', restore);
}

/** 应用后文档又被手动改过(内容不再是应用版本),自动撤销会吞掉人工修改 */
function isStale(message: PanelMessage): boolean {
  return message.applied && props.document !== message.doc;
}

function togglePreview(message: PanelMessage) {
  if (expandedIds.value.has(message.id)) {
    expandedIds.value.delete(message.id);
  } else {
    expandedIds.value.add(message.id);
  }
}

watch([messages, sending], scrollToEnd, { deep: true });

onBeforeUnmount(stopTimer);
</script>

<template>
  <SagAiPanel
    ref="panelRef"
    title="AI 助手"
    placeholder="让 AI 帮你改这篇文档…"
    :disabled="!hasDocument"
    :busy="sending"
    @send="send"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <!-- 未打开文档 -->
      <div
        v-if="!hasDocument"
        class="flex h-full items-center justify-center px-4 text-center text-xs text-muted-foreground"
      >
        先从左侧打开一篇文档，再和 AI 对话。
      </div>

      <template v-else>
        <!-- 空会话提示 -->
        <div
          v-if="messages.length === 0"
          class="rounded-lg border border-dashed p-4 text-xs leading-6 text-muted-foreground"
        >
          和 AI 聊聊这篇文档，比如：<br />
          · 帮我把第 2 节润色得更紧凑<br />
          · 在状态机里加一个「过期」状态<br />
          · 总结全文要点，列出还没写完的部分
        </div>

        <div
          v-for="message of messages"
          :key="message.id"
          class="flex flex-col gap-1"
          :class="message.role === 'user' ? 'items-end' : 'items-start'"
        >
          <div
            v-if="message.role === 'user'"
            class="max-w-[85%] rounded-2xl bg-muted px-3 py-2 text-sm whitespace-pre-wrap break-words"
          >
            {{ message.text }}
          </div>

          <template v-else>
            <div class="docs-ai-markdown w-full min-w-0 text-sm leading-relaxed">
              <MessageResponse :content="message.text" />
            </div>
            <p v-if="message.modelLabel" class="font-mono text-[11px] text-muted-foreground">
              {{ message.modelLabel }}
            </p>

            <!-- 文档修改卡 -->
            <div v-if="message.doc !== null" class="w-full space-y-2 rounded-lg border bg-muted/30 p-2.5">
              <p class="text-xs text-muted-foreground">
                AI 生成了新的文档版本（{{ message.doc.length }} 字，当前 {{ props.document.length }} 字）
              </p>
              <pre
                v-if="expandedIds.has(message.id)"
                class="max-h-64 overflow-auto rounded-md bg-muted p-2 font-mono text-xs whitespace-pre-wrap"
                >{{ message.doc }}</pre>
              <div class="flex flex-wrap items-center gap-2">
                <Button v-if="!message.applied" size="sm" @click="applyDoc(message)"> 应用到文档 </Button>
                <Button size="sm" variant="outline" @click="togglePreview(message)">
                  {{ expandedIds.has(message.id) ? '收起预览' : '预览' }}
                </Button>
                <Button
                  v-if="message.applied && !isStale(message)"
                  size="sm"
                  variant="outline"
                  @click="undoApply(message)"
                >
                  撤销应用
                </Button>
                <span v-else-if="message.applied" class="text-xs text-muted-foreground">
                  文档已在应用后被手动修改，无法自动撤销
                </span>
              </div>
              <p v-if="message.applied" class="text-xs text-emerald-600 dark:text-emerald-400">
                已应用，⌘S 保存后写入磁盘
              </p>
            </div>
          </template>
        </div>

        <div v-if="sending" class="flex items-center gap-2 text-xs text-muted-foreground">
          <Spinner class="h-3.5 w-3.5" />
          <span>正在思考</span>
          <span class="tabular-nums">{{ elapsedSeconds }}s</span>
        </div>
      </template>
    </div>
  </SagAiPanel>
</template>

<style>
.docs-ai-markdown p + p {
  margin-top: 0.5em;
}
</style>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { NodeViewContent, NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3';
import { Code2, Eye, Maximize2 } from 'lucide-vue-next';
import { useAppStore } from '@/stores/app';
import { renderMermaidDiagram } from './mermaid-render';
import MermaidZoomOverlay from './mermaid-zoom-overlay.vue';

/**
 * 代码块节点视图:mermaid 语言时是一张"图表卡"——默认显示渲染结果,
 * 头部按钮或光标进入块内切换到源码编辑,改完自动回到图;点图放大全屏预览。
 * 其余语言与原生代码块一致。节点结构不变,markdown 往返不受影响。
 */
const props = defineProps(nodeViewProps);

const appStore = useAppStore();

const isMermaid = computed(() => props.node.attrs.language === 'mermaid');
const source = computed(() => props.node.textContent);
const isEmpty = computed(() => !source.value.trim());

const systemDark = ref(false);
let media: MediaQueryList | null = null;

const isDark = computed(() => {
  const theme = appStore.settings.theme;
  if (theme === 'dark') {
    return true;
  }
  if (theme === 'light') {
    return false;
  }
  return systemDark.value;
});

const svg = ref('');
const error = ref('');
const rendering = ref(false);
const zoomOpen = ref(false);

// 用户手动选的模式;空块强制代码模式。光标进入块内自动显示源码,
// 但"查看图表"按钮的显式切换优先于光标跟随(suppressAuto 压住自动逻辑,
// 直到光标重新进入块内,或在隐藏代码里敲了字——防止看不见的盲打)
const stickyMode = ref<'diagram' | 'code'>('diagram');
const cursorInside = ref(false);
const suppressAuto = ref(false);
const mode = computed(() => {
  if (isEmpty.value || (cursorInside.value && !suppressAuto.value)) {
    return 'code';
  }
  return stickyMode.value;
});

let timer: ReturnType<typeof setTimeout> | null = null;
let seq = 0;
let disposed = false;

async function render() {
  if (!source.value.trim()) {
    svg.value = '';
    error.value = '';
    rendering.value = false;
    return;
  }
  const current = ++seq;
  rendering.value = true;
  const result = await renderMermaidDiagram(source.value, isDark.value);
  if (disposed || current !== seq) {
    return;
  }
  rendering.value = false;
  if (result.ok) {
    svg.value = result.svg;
    error.value = '';
  } else {
    svg.value = '';
    error.value = result.message;
  }
}

function scheduleRender() {
  if (timer) {
    clearTimeout(timer);
  }
  timer = setTimeout(() => {
    timer = null;
    void render();
  }, 500);
}

function nodePos(): number | undefined {
  return typeof props.getPos === 'function' ? props.getPos() : undefined;
}

function updateCursorInside() {
  const pos = nodePos();
  if (typeof pos !== 'number') {
    return;
  }
  const { from, to } = props.editor.state.selection;
  const end = pos + props.node.nodeSize;
  // NodeSelection(整块选中)不算在内,否则点图放大时会被切到源码
  const inside = from > pos && to < end;
  if (inside && !cursorInside.value) {
    suppressAuto.value = false;
  }
  cursorInside.value = inside;
}

function switchToCode() {
  stickyMode.value = 'code';
  void nextTick(() => {
    const pos = nodePos();
    if (typeof pos === 'number') {
      props.editor.chain().focus().setTextSelection(pos + 1).run();
    }
  });
}

function switchToDiagram() {
  stickyMode.value = 'diagram';
  suppressAuto.value = true;
}

function onTransaction({ transaction }: { transaction: { docChanged: boolean } }) {
  // 隐藏代码里出现编辑时立刻切回源码,避免看不见的盲打
  if (!transaction.docChanged || !suppressAuto.value) {
    return;
  }
  updateCursorInside();
  if (cursorInside.value) {
    suppressAuto.value = false;
  }
}

// 切到 mermaid 语言立即出图;源码/主题变化防抖重渲
watch(isMermaid, value => {
  if (value) {
    void render();
  } else {
    svg.value = '';
    error.value = '';
    zoomOpen.value = false;
  }
}, { immediate: true });

watch([source, isDark], () => {
  if (isMermaid.value) {
    scheduleRender();
  }
});

function onMediaChange(event: MediaQueryListEvent) {
  systemDark.value = event.matches;
}

onMounted(() => {
  media = window.matchMedia('(prefers-color-scheme: dark)');
  systemDark.value = media.matches;
  media.addEventListener('change', onMediaChange);
  props.editor.on('selectionUpdate', updateCursorInside);
  props.editor.on('transaction', onTransaction);
  updateCursorInside();
});

onBeforeUnmount(() => {
  disposed = true;
  if (timer) {
    clearTimeout(timer);
  }
  media?.removeEventListener('change', onMediaChange);
  props.editor.off('selectionUpdate', updateCursorInside);
  props.editor.off('transaction', onTransaction);
});
</script>

<template>
  <NodeViewWrapper
    class="sag-mermaid-block"
    :data-mermaid="isMermaid || undefined"
    :data-mode="isMermaid ? mode : undefined"
  >
    <!-- mermaid 图表卡头部:标签 + 模式切换 + 放大 -->
    <div
      v-if="isMermaid"
      class="sag-mermaid-head"
    >
      <span class="sag-mermaid-title">Mermaid</span>
      <button
        v-if="mode === 'diagram'"
        type="button"
        class="sag-mermaid-btn"
        aria-label="编辑源码"
        title="编辑源码"
        @mousedown.prevent
        @click="switchToCode"
      >
        <Code2 class="h-3.5 w-3.5" />
      </button>
      <button
        v-else
        type="button"
        class="sag-mermaid-btn"
        aria-label="查看图表"
        title="查看图表"
        @mousedown.prevent
        @click="switchToDiagram"
      >
        <Eye class="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        class="sag-mermaid-btn"
        aria-label="放大图表"
        title="放大图表"
        :disabled="!svg"
        @mousedown.prevent
        @click="zoomOpen = true"
      >
        <Maximize2 class="h-3.5 w-3.5" />
      </button>
    </div>

    <pre v-show="!isMermaid || mode === 'code'"><NodeViewContent as="code" /></pre>

    <template v-if="isMermaid">
      <!-- 代码模式:源码在上,下面按状态给提示/报错;不显示图 -->
      <template v-if="mode === 'code'">
        <div
          v-if="isEmpty"
          class="sag-mermaid-hint"
        >
          输入 mermaid 源码，点击右上角切换为图表
        </div>
        <div
          v-else-if="error"
          class="sag-mermaid-error"
          :title="error"
        >
          {{ error }}
        </div>
      </template>

      <!-- 图表模式:渲染结果或失败面板,不显示源码 -->
      <div
        v-else-if="error"
        class="sag-mermaid-fail"
      >
        <p
          class="truncate"
          :title="error"
        >
          图表渲染失败：{{ error }}
        </p>
        <button
          type="button"
          class="sag-mermaid-fail-action"
          @mousedown.prevent
          @click="switchToCode"
        >
          编辑源码
        </button>
      </div>

      <div
        v-else
        class="sag-mermaid-diagram"
        :class="{ 'is-rendering': rendering }"
        title="点击放大"
        @click="svg && (zoomOpen = true)"
      >
        <div
          class="sag-mermaid-diagram-inner"
          v-html="svg"
        />
      </div>

      <MermaidZoomOverlay
        v-if="zoomOpen && svg"
        :svg="svg"
        @close="zoomOpen = false"
      />
    </template>
  </NodeViewWrapper>
</template>

<style>
/* mermaid 图表卡:头部 + 图/码一体的边框容器;普通代码块的包装层不带任何样式 */
.sag-rich-editor .ProseMirror .sag-mermaid-block[data-mermaid] {
  margin-top: 0.6em;
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  overflow: hidden;
}

.sag-rich-editor .ProseMirror .sag-mermaid-block[data-mermaid] pre {
  margin: 0;
  border-radius: 0;
  background: var(--muted);
}

.sag-rich-editor .ProseMirror .sag-mermaid-block[data-mermaid].ProseMirror-selectednode {
  outline: 2px solid var(--ring);
  outline-offset: 1px;
}

.sag-mermaid-head {
  display: flex;
  align-items: center;
  gap: 0.125rem;
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid var(--border);
  background: color-mix(in oklab, var(--muted) 45%, transparent);
}

.sag-mermaid-title {
  flex: 1;
  font-size: 0.75rem;
  line-height: 1.5;
  color: var(--muted-foreground);
}

.sag-mermaid-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 0.375rem;
  color: var(--muted-foreground);
  transition: color 120ms ease, background-color 120ms ease;
}

.sag-mermaid-btn:hover:not(:disabled) {
  color: var(--foreground);
  background: var(--accent);
}

.sag-mermaid-btn:disabled {
  opacity: 0.4;
  pointer-events: none;
}

.sag-mermaid-diagram {
  display: flex;
  justify-content: center;
  overflow-x: auto;
  padding: 1rem;
  cursor: zoom-in;
  transition: opacity 150ms ease;
}

.sag-mermaid-diagram.is-rendering {
  opacity: 0.55;
}

.sag-mermaid-diagram-inner {
  max-width: 100%;
}

.sag-mermaid-diagram-inner svg {
  max-width: 100%;
  height: auto;
}

.sag-mermaid-hint {
  padding: 0.5rem 1rem;
  border-top: 1px solid var(--border);
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.sag-mermaid-error {
  padding: 0.5rem 1rem;
  border-top: 1px solid var(--border);
  font-size: 0.75rem;
  font-family: var(--font-mono, ui-monospace, monospace);
  color: var(--destructive);
  background: color-mix(in oklab, var(--destructive) 8%, transparent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sag-mermaid-fail {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  font-size: 0.75rem;
  color: var(--destructive);
  background: color-mix(in oklab, var(--destructive) 8%, transparent);
}

.sag-mermaid-fail p {
  flex: 1;
  min-width: 0;
}

.sag-mermaid-fail-action {
  flex-shrink: 0;
  font-size: 0.75rem;
  color: var(--foreground);
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>

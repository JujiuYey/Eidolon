<script setup lang="ts">
import Placeholder from '@tiptap/extension-placeholder';
import Table from '@tiptap/extension-table';
import TableCell from '@tiptap/extension-table-cell';
import TableHeader from '@tiptap/extension-table-header';
import TableRow from '@tiptap/extension-table-row';
import StarterKit from '@tiptap/starter-kit';
import type { Editor } from '@tiptap/vue-3';
import { EditorContent, useEditor } from '@tiptap/vue-3';
import {
  Bold,
  Code,
  Heading1,
  Heading2,
  Heading3,
  Italic,
  List,
  ListOrdered,
  Minus,
  Quote,
  Redo2,
  SquareCode,
  Strikethrough,
  Table as TableIcon,
  Undo2,
} from 'lucide-vue-next';
import { Markdown } from 'tiptap-markdown';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Separator } from '@/components/ui/separator';

import { editorLowlight } from './lowlight';
import { MermaidCodeBlock } from './mermaid-code-block';
import { collectTocItems, type RichEditorTocItem } from './toc';

const props = withDefaults(
  defineProps<{
    placeholder?: string;
    /** Notion 式全宽阅读:内容列去掉限宽,开关按钮由使用方放在 actions 插槽 */
    fullWidth?: boolean;
  }>(),
  {
    placeholder: '开始写吧，⌘S / Ctrl+S 保存。',
    fullWidth: false,
  },
);

/**
 * 共享富文本编辑器：TipTap 所见即所得，v-model 是 **markdown 字符串**。
 * 打开文件 → markdown 解析进文档；编辑 → 序列化回 markdown 抛出。
 * 落盘格式永远是 .md，脏状态与保存逻辑由使用方基于 v-model 判断。
 */

const content = defineModel<string>({ required: true });

// 防回环：外部 setContent 触发的 onUpdate 不再向外 emit
let lastEmitted = '';
let applyingExternal = false;

const editor = useEditor({
  content: content.value,
  extensions: [
    StarterKit.configure({ codeBlock: false }),
    MermaidCodeBlock.configure({ lowlight: editorLowlight }),
    Placeholder.configure({ placeholder: props.placeholder }),
    Markdown.configure({
      html: false,
      breaks: true,
      linkify: false,
      transformPastedText: true,
      transformCopiedText: true,
    }),
    Table.configure({ resizable: false }),
    TableRow,
    TableHeader,
    TableCell,
  ],
  editorProps: {
    attributes: {
      class: 'sag-rich-editor-content focus:outline-none',
    },
  },
  onUpdate({ editor: current }) {
    if (applyingExternal) {
      return;
    }
    lastEmitted = current.storage.markdown.getMarkdown();
    content.value = lastEmitted;
  },
});

// 外部换文件（v-model 整体替换）时重载文档
watch(content, value => {
  if (value === lastEmitted || !editor.value) {
    return;
  }
  applyingExternal = true;
  editor.value.commands.setContent(value || '', false);
  applyingExternal = false;
});

const canUndo = ref(false);
const canRedo = ref(false);

/** 目录数据:每次事务后重算,父组件通过模板 ref 读取并渲染目录面板 */
const tocItems = ref<RichEditorTocItem[]>([]);

// useEditor 是一次性 shallowRef,撤销栈/文档变化不会触发响应式,手动监听事务同步
function syncEditorState() {
  canUndo.value = editor.value?.can().undo() ?? false;
  canRedo.value = editor.value?.can().redo() ?? false;
  const current = editor.value;
  tocItems.value = current ? collectTocItems(current.state.doc) : [];
}

onMounted(() =>
  nextTick(() => {
    const current = editor.value;
    if (current) {
      current.on('transaction', syncEditorState);
      syncEditorState();
    }
  }),
);

onBeforeUnmount(() => {
  editor.value?.off('transaction', syncEditorState);
  editor.value?.destroy();
});

/** 目录点击跳转:找到标题对应的 DOM 块,平滑滚到顶部(不移动光标) */
function jumpToHeading(pos: number) {
  const dom = editor.value?.view.nodeDOM(pos);
  if (dom instanceof HTMLElement) {
    dom.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

defineExpose({
  tocItems,
  jumpToHeading,
});

function chain() {
  return editor.value?.chain().focus();
}

interface ToolbarToggle {
  key: string;
  icon: typeof Bold;
  label: string;
  active: boolean;
  run: () => void;
}

const toggles = computed<ToolbarToggle[]>(() => {
  const e = editor.value;
  if (!e) {
    return [];
  }
  const make = (key: string, icon: typeof Bold, label: string, mark: string, attrs: Record<string, unknown> = {}) => ({
    key,
    icon,
    label,
    active: e.isActive(mark, attrs),
    run: () => e.chain().focus().toggleMark(mark, attrs).run(),
  });
  return [
    make('bold', Bold, '加粗', 'bold'),
    make('italic', Italic, '斜体', 'italic'),
    make('strike', Strikethrough, '删除线', 'strike'),
    make('code', Code, '行内代码', 'code'),
    {
      key: 'bullet',
      icon: List,
      label: '无序列表',
      active: e.isActive('bulletList'),
      run: () => e.chain().focus().toggleBulletList().run(),
    },
    {
      key: 'ordered',
      icon: ListOrdered,
      label: '有序列表',
      active: e.isActive('orderedList'),
      run: () => e.chain().focus().toggleOrderedList().run(),
    },
    {
      key: 'quote',
      icon: Quote,
      label: '引用',
      active: e.isActive('blockquote'),
      run: () => e.chain().focus().toggleBlockquote().run(),
    },
    {
      key: 'codeblock',
      icon: SquareCode,
      label: '代码块',
      active: e.isActive('codeBlock'),
      run: () => e.chain().focus().toggleCodeBlock().run(),
    },
    { key: 'hr', icon: Minus, label: '分隔线', active: false, run: () => e.chain().focus().setHorizontalRule().run() },
  ];
});

const headings = computed(() => {
  const e = editor.value;
  if (!e) {
    return [];
  }
  return [
    { key: 'h1', icon: Heading1, label: '标题一', level: 1 as const, active: e.isActive('heading', { level: 1 }) },
    { key: 'h2', icon: Heading2, label: '标题二', level: 2 as const, active: e.isActive('heading', { level: 2 }) },
    { key: 'h3', icon: Heading3, label: '标题三', level: 3 as const, active: e.isActive('heading', { level: 3 }) },
  ];
});

const tableActions = [
  {
    key: 'insert',
    label: '插入表格（3×3）',
    run: (e: Editor) => e.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  { key: 'row-above', label: '上方插入行', run: (e: Editor) => e.chain().focus().addRowBefore().run() },
  { key: 'row-below', label: '下方插入行', run: (e: Editor) => e.chain().focus().addRowAfter().run() },
  { key: 'col-left', label: '左侧插入列', run: (e: Editor) => e.chain().focus().addColumnBefore().run() },
  { key: 'col-right', label: '右侧插入列', run: (e: Editor) => e.chain().focus().addColumnAfter().run() },
  { key: 'del-row', label: '删除当前行', run: (e: Editor) => e.chain().focus().deleteRow().run() },
  { key: 'del-col', label: '删除当前列', run: (e: Editor) => e.chain().focus().deleteColumn().run() },
  { key: 'del-table', label: '删除整个表格', run: (e: Editor) => e.chain().focus().deleteTable().run() },
];
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <!-- 工具栏:最左撤销重做 + 格式按钮,右侧 actions 插槽放各页面自己的操作(AI/保存等) -->
    <div class="flex shrink-0 flex-wrap items-center gap-0.5 border-b px-2 py-1.5">
      <Button
        aria-label="撤销"
        :disabled="!canUndo"
        class="h-8 w-8"
        size="icon"
        variant="ghost"
        @click="chain()?.undo().run()"
      >
        <Undo2 class="h-4 w-4" />
      </Button>
      <Button
        aria-label="重做"
        :disabled="!canRedo"
        class="h-8 w-8"
        size="icon"
        variant="ghost"
        @click="chain()?.redo().run()"
      >
        <Redo2 class="h-4 w-4" />
      </Button>

      <Separator orientation="vertical" class="mx-1 !h-5" />

      <Button
        v-for="heading of headings"
        :key="heading.key"
        :aria-label="heading.label"
        :variant="heading.active ? 'secondary' : 'ghost'"
        class="h-8 w-8"
        size="icon"
        @click="chain()?.toggleHeading({ level: heading.level }).run()"
      >
        <component :is="heading.icon" class="h-4 w-4" />
      </Button>

      <Separator orientation="vertical" class="mx-1 !h-5" />

      <Button
        v-for="item of toggles"
        :key="item.key"
        :aria-label="item.label"
        :variant="item.active ? 'secondary' : 'ghost'"
        class="h-8 w-8"
        size="icon"
        @click="item.run()"
      >
        <component :is="item.icon" class="h-4 w-4" />
      </Button>

      <Separator orientation="vertical" class="mx-1 !h-5" />

      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button
            :variant="editor?.isActive('table') ? 'secondary' : 'ghost'"
            aria-label="表格"
            class="h-8 w-8"
            size="icon"
          >
            <TableIcon class="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem
            v-for="action of tableActions.slice(0, 1)"
            :key="action.key"
            @click="editor && action.run(editor)"
          >
            {{ action.label }}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            v-for="action of tableActions.slice(1)"
            :key="action.key"
            :disabled="!editor?.isActive('table')"
            @click="editor && action.run(editor)"
          >
            {{ action.label }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <div class="ml-auto flex items-center gap-1.5">
        <slot name="actions" />
      </div>
    </div>

    <!-- 编辑区:默认限宽 3xl,全宽模式撑满 -->
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="sag-rich-editor mx-auto px-6 py-5" :class="fullWidth ? 'max-w-none' : 'max-w-3xl'">
        <EditorContent :editor="editor" />
      </div>
    </div>
  </div>
</template>

<style>
.sag-rich-editor .ProseMirror {
  min-height: 100%;
  outline: none;
  font-size: 0.9375rem;
  line-height: 1.75;
}

.sag-rich-editor .ProseMirror > * + * {
  margin-top: 0.6em;
}

.sag-rich-editor .ProseMirror h1,
.sag-rich-editor .ProseMirror h2,
.sag-rich-editor .ProseMirror h3 {
  font-weight: 600;
  line-height: 1.3;
  margin-top: 1.2em;
  /* 目录跳转定位时给标题留出上边距 */
  scroll-margin-top: 0.75rem;
}

.sag-rich-editor .ProseMirror h1 {
  font-size: 1.5rem;
}

.sag-rich-editor .ProseMirror h2 {
  font-size: 1.25rem;
}

.sag-rich-editor .ProseMirror h3 {
  font-size: 1.075rem;
}

.sag-rich-editor .ProseMirror ul,
.sag-rich-editor .ProseMirror ol {
  padding-left: 1.4em;
}

.sag-rich-editor .ProseMirror ul {
  list-style: disc;
}

.sag-rich-editor .ProseMirror ol {
  list-style: decimal;
}

.sag-rich-editor .ProseMirror blockquote {
  border-left: 3px solid var(--border);
  padding-left: 0.9em;
  color: var(--muted-foreground);
}

/* 行内代码 chip：主题色文字 + 同色淡底；:not(pre) 把代码块里的 code 排除在外 */
.sag-rich-editor .ProseMirror :not(pre) > code {
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 0.85em;
  color: var(--primary);
  background: color-mix(in oklab, var(--primary) 10%, transparent);
  border-radius: 0.25rem;
  padding: 0.15em 0.35em;
}

/* 暗色下主题色原值偏暗，往白里提一档保证可读 */
.dark .sag-rich-editor .ProseMirror :not(pre) > code {
  color: color-mix(in oklab, var(--primary) 70%, white);
  background: color-mix(in oklab, var(--primary) 16%, transparent);
}

.sag-rich-editor .ProseMirror pre {
  background: var(--muted);
  border-radius: 0.5rem;
  padding: 0.85em 1em;
}

.sag-rich-editor .ProseMirror pre code {
  font-size: 0.85em;
  line-height: 1.6;
}

/* 代码块语法高亮:lowlight 往 code 里打 hljs-* 装饰 span,配色取 GitHub Light/Dark 两套 */
.sag-rich-editor .ProseMirror .hljs-comment,
.sag-rich-editor .ProseMirror .hljs-quote,
.sag-rich-editor .ProseMirror .hljs-code,
.sag-rich-editor .ProseMirror .hljs-meta {
  color: #6a737d;
}

.sag-rich-editor .ProseMirror .hljs-keyword,
.sag-rich-editor .ProseMirror .hljs-doctag,
.sag-rich-editor .ProseMirror .hljs-selector-tag {
  color: #d73a49;
}

.sag-rich-editor .ProseMirror .hljs-name,
.sag-rich-editor .ProseMirror .hljs-section,
.sag-rich-editor .ProseMirror .hljs-addition {
  color: #22863a;
}

.sag-rich-editor .ProseMirror .hljs-string,
.sag-rich-editor .ProseMirror .hljs-regexp {
  color: #032f62;
}

.sag-rich-editor .ProseMirror .hljs-number,
.sag-rich-editor .ProseMirror .hljs-literal,
.sag-rich-editor .ProseMirror .hljs-bullet,
.sag-rich-editor .ProseMirror .hljs-link,
.sag-rich-editor .ProseMirror .hljs-symbol,
.sag-rich-editor .ProseMirror .hljs-operator,
.sag-rich-editor .ProseMirror .hljs-attr,
.sag-rich-editor .ProseMirror .hljs-attribute,
.sag-rich-editor .ProseMirror .hljs-property {
  color: #005cc5;
}

.sag-rich-editor .ProseMirror .hljs-variable,
.sag-rich-editor .ProseMirror .hljs-template-variable,
.sag-rich-editor .ProseMirror .hljs-built_in {
  color: #e36209;
}

.sag-rich-editor .ProseMirror .hljs-title,
.sag-rich-editor .ProseMirror .hljs-type,
.sag-rich-editor .ProseMirror .hljs-selector-id,
.sag-rich-editor .ProseMirror .hljs-selector-class,
.sag-rich-editor .ProseMirror .hljs-selector-attr,
.sag-rich-editor .ProseMirror .hljs-selector-pseudo {
  color: #6f42c1;
}

.sag-rich-editor .ProseMirror .hljs-deletion {
  color: #b31d28;
}

.sag-rich-editor .ProseMirror .hljs-emphasis {
  font-style: italic;
}

.sag-rich-editor .ProseMirror .hljs-strong {
  font-weight: 600;
}

.dark .sag-rich-editor .ProseMirror .hljs-comment,
.dark .sag-rich-editor .ProseMirror .hljs-quote,
.dark .sag-rich-editor .ProseMirror .hljs-code,
.dark .sag-rich-editor .ProseMirror .hljs-meta {
  color: #8b949e;
}

.dark .sag-rich-editor .ProseMirror .hljs-keyword,
.dark .sag-rich-editor .ProseMirror .hljs-doctag,
.dark .sag-rich-editor .ProseMirror .hljs-selector-tag {
  color: #ff7b72;
}

.dark .sag-rich-editor .ProseMirror .hljs-name,
.dark .sag-rich-editor .ProseMirror .hljs-section,
.dark .sag-rich-editor .ProseMirror .hljs-addition {
  color: #7ee787;
}

.dark .sag-rich-editor .ProseMirror .hljs-string,
.dark .sag-rich-editor .ProseMirror .hljs-regexp {
  color: #a5d6ff;
}

.dark .sag-rich-editor .ProseMirror .hljs-number,
.dark .sag-rich-editor .ProseMirror .hljs-literal,
.dark .sag-rich-editor .ProseMirror .hljs-bullet,
.dark .sag-rich-editor .ProseMirror .hljs-link,
.dark .sag-rich-editor .ProseMirror .hljs-symbol,
.dark .sag-rich-editor .ProseMirror .hljs-operator,
.dark .sag-rich-editor .ProseMirror .hljs-attr,
.dark .sag-rich-editor .ProseMirror .hljs-attribute,
.dark .sag-rich-editor .ProseMirror .hljs-property {
  color: #79c0ff;
}

.dark .sag-rich-editor .ProseMirror .hljs-variable,
.dark .sag-rich-editor .ProseMirror .hljs-template-variable,
.dark .sag-rich-editor .ProseMirror .hljs-built_in {
  color: #ffa657;
}

.dark .sag-rich-editor .ProseMirror .hljs-title,
.dark .sag-rich-editor .ProseMirror .hljs-type,
.dark .sag-rich-editor .ProseMirror .hljs-selector-id,
.dark .sag-rich-editor .ProseMirror .hljs-selector-class,
.dark .sag-rich-editor .ProseMirror .hljs-selector-attr,
.dark .sag-rich-editor .ProseMirror .hljs-selector-pseudo {
  color: #d2a8ff;
}

.dark .sag-rich-editor .ProseMirror .hljs-deletion {
  color: #ffdcd7;
}

.sag-rich-editor .ProseMirror hr {
  border: none;
  border-top: 1px solid var(--border);
}

.sag-rich-editor .ProseMirror table {
  border-collapse: collapse;
  width: 100%;
  table-layout: fixed;
  overflow: hidden;
}

.sag-rich-editor .ProseMirror th,
.sag-rich-editor .ProseMirror td {
  border: 1px solid var(--border);
  padding: 0.45em 0.7em;
  vertical-align: top;
  position: relative;
}

.sag-rich-editor .ProseMirror th {
  background: var(--muted);
  font-weight: 600;
  text-align: left;
}

.sag-rich-editor .ProseMirror .selectedCell {
  background: var(--accent);
}

.sag-rich-editor .ProseMirror p.is-editor-empty:first-child::before {
  content: attr(data-placeholder);
  color: var(--muted-foreground);
  float: left;
  height: 0;
  pointer-events: none;
}
</style>

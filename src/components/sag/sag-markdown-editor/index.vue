<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import MarkdownRender from 'markstream-vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { SagMarkdownGuide, SagMarkdownSnippet } from './types';

interface Props {
  badgeLabel?: string;
  description?: string;
  editorHeightClass?: string;
  editorHint?: string;
  editorLabel?: string;
  emptyPreviewContent?: string;
  guide?: SagMarkdownGuide | null;
  placeholder?: string;
  previewHint?: string;
  previewLabel?: string;
  snippets?: SagMarkdownSnippet[];
  title?: string;
}

const props = withDefaults(defineProps<Props>(), {
  badgeLabel: 'Markdown 编辑',
  description: '',
  editorHeightClass: 'h-[28rem]',
  editorHint: '',
  editorLabel: '正文',
  emptyPreviewContent: [
    '## 实时预览',
    '',
    '这里会展示当前 Markdown 内容的排版效果。',
  ].join('\n'),
  guide: null,
  placeholder: '在这里编写内容，支持 Markdown 结构，比如标题、列表、引用和代码块。',
  previewHint: '用来检查结构和可读性。',
  previewLabel: 'Markdown 预览',
  snippets: () => [
    {
      label: '二级标题',
      value: '## 小节标题',
    },
    {
      label: '无序列表',
      value: '- 要点一\n- 要点二\n- 要点三',
    },
    {
      label: '有序步骤',
      value: '1. 第一步\n2. 第二步\n3. 第三步',
    },
    {
      label: '引用说明',
      value: '> 补充说明',
    },
    {
      label: '代码块',
      value: '```text\n在这里写示例内容\n```',
    },
  ],
  title: '支持 Markdown 书写习惯，建议用标题、列表和代码块组织结构。',
});

const content = defineModel<string>({ required: true });

const previewMode = ref<'edit' | 'preview' | 'split'>('split');
const editorRef = ref<HTMLTextAreaElement | null>(null);

const PREVIEW_CUSTOM_ID = 'sag-markdown-editor-preview';
const resolvedDescription = computed(() => {
  return props.guide?.description || props.description;
});

const previewContent = computed(() => {
  if (content.value.trim()) {
    return content.value;
  }

  return props.guide?.emptyPreviewContent || props.emptyPreviewContent;
});

const charCount = computed(() => content.value.length);
const lineCount = computed(() => {
  return content.value ? content.value.split(/\r?\n/).length : 0;
});

const resolvedSnippets = computed(() => {
  return [
    ...(props.guide?.primarySnippet ? [props.guide.primarySnippet] : []),
    ...props.snippets,
  ];
});

async function insertSnippet(snippet: string) {
  const textarea = editorRef.value;
  const currentValue = content.value;

  if (!textarea) {
    content.value = currentValue ? `${currentValue}\n\n${snippet}` : snippet;
    return;
  }

  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const before = currentValue.slice(0, start);
  const after = currentValue.slice(end);

  const prefix = before.length > 0 && !before.endsWith('\n') ? '\n\n' : '';
  const suffix = after.length > 0 && !after.startsWith('\n') ? '\n\n' : '';

  content.value = `${before}${prefix}${snippet}${suffix}${after}`;

  await nextTick();

  const nextCursor = before.length + prefix.length + snippet.length;
  textarea.focus();
  textarea.setSelectionRange(nextCursor, nextCursor);
}
</script>

<template>
  <div class="space-y-4">
    <div class="rounded-xl border bg-muted/20 p-4">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div class="space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <Badge variant="secondary">
              {{ badgeLabel }}
            </Badge>
            <Badge variant="outline">
              {{ lineCount }} 行
            </Badge>
            <Badge variant="outline">
              {{ charCount }} 字符
            </Badge>
          </div>
          <p class="text-sm text-foreground">
            {{ title }}
          </p>
          <p v-if="resolvedDescription" class="text-xs text-muted-foreground">
            {{ resolvedDescription }}
          </p>
        </div>

        <Tabs v-model="previewMode" class="w-full lg:w-auto">
          <TabsList class="grid w-full grid-cols-3 lg:w-[18rem]">
            <TabsTrigger value="edit">
              编辑
            </TabsTrigger>
            <TabsTrigger value="split">
              分栏
            </TabsTrigger>
            <TabsTrigger value="preview">
              预览
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      <div class="mt-4 flex flex-wrap gap-2">
        <Button
          v-for="snippet of resolvedSnippets"
          :key="snippet.label"
          type="button"
          size="sm"
          variant="outline"
          @click="insertSnippet(snippet.value)"
        >
          {{ snippet.label }}
        </Button>
      </div>
    </div>

    <div
      class="grid gap-4"
      :class="previewMode === 'split'
        ? 'grid-cols-1 xl:grid-cols-[minmax(0,1.1fr)_minmax(20rem,0.9fr)]'
        : 'grid-cols-1'"
    >
      <div
        v-if="previewMode !== 'preview'"
        class="overflow-hidden rounded-xl bg-background p-1"
      >
        <div v-if="editorLabel || editorHint" class="space-y-1 px-2 pb-2 pt-1">
          <p v-if="editorLabel" class="text-sm font-medium">
            {{ editorLabel }}
          </p>
          <p v-if="editorHint" class="text-xs text-muted-foreground">
            {{ editorHint }}
          </p>
        </div>
        <textarea
          ref="editorRef"
          v-model="content"
          class="border-input placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:bg-input/30 flex w-full rounded-md border bg-transparent px-3 py-3 font-mono text-sm leading-6 shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px]"
          :class="[
            editorHeightClass,
          ]"
          :placeholder="placeholder"
        />
      </div>

      <div
        v-if="previewMode !== 'edit'"
        class="overflow-hidden rounded-xl border bg-background p-2"
      >
        <div v-if="previewLabel || previewHint" class="space-y-1 px-2 pb-2 pt-1">
          <p v-if="previewLabel" class="text-sm font-medium">
            {{ previewLabel }}
          </p>
          <p v-if="previewHint" class="text-xs text-muted-foreground">
            {{ previewHint }}
          </p>
        </div>
        <div class="overflow-auto" :class="editorHeightClass">
          <MarkdownRender
            :content="previewContent"
            :custom-id="PREVIEW_CUSTOM_ID"
          />
        </div>
      </div>
    </div>
  </div>
</template>

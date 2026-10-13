<script setup lang="ts">
import type { RichEditorTocItem } from '@/components/sag/sag-rich-editor/toc';
import { ScrollArea } from '@/components/ui/scroll-area';

/**
 * 文档目录面板:列出当前文档的 h1–h3,点击通知父组件跳转。
 * 缩进档位与左侧文档树一致(每级 14px)。
 */
defineProps<{
  items: RichEditorTocItem[];
}>();

const emit = defineEmits<{
  (e: 'jump', pos: number): void;
}>();

function indentFor(level: 1 | 2 | 3) {
  return { paddingLeft: `${8 + (level - 1) * 14}px` };
}
</script>

<template>
  <aside class="flex h-full w-full min-w-0 flex-col overflow-hidden rounded-xl border bg-card">
    <div class="flex items-center justify-between px-4 py-3">
      <h2 class="text-sm font-semibold">目录</h2>
      <span class="text-xs tabular-nums text-muted-foreground">{{ items.length }}</span>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <p v-if="items.length === 0" class="px-2 py-6 text-center text-xs text-muted-foreground">
        这篇文档还没有标题，用工具栏加上 h1–h3 后会出现在这里。
      </p>
      <div v-else class="space-y-0.5 px-2 pb-2">
        <button
          v-for="(item, index) of items"
          :key="`${item.pos}-${index}`"
          class="block w-full truncate rounded-lg py-1.5 pr-1 text-left text-sm transition-colors hover:bg-muted/60"
          :class="item.level === 1 ? 'font-medium text-foreground' : 'text-muted-foreground'"
          :style="indentFor(item.level)"
          :title="item.text"
          type="button"
          @click="emit('jump', item.pos)"
        >
          {{ item.text || '（无标题）' }}
        </button>
      </div>
    </ScrollArea>
  </aside>
</template>

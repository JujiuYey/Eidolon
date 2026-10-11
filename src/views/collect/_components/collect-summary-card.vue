<script setup lang="ts">
import { Loader2, Sparkles, X } from 'lucide-vue-next';
import MarkdownRender from 'markstream-vue';

import { Button } from '@/components/ui/button';
import type { AiSummaryState } from '@/stores/collect';

defineProps<{
  summary: AiSummaryState;
  /** markstream-vue 渲染缓存 id，同页多张摘要卡不能撞车 */
  customId: string;
}>();

const emit = defineEmits<{
  (e: 'dismiss'): void;
}>();
</script>

<template>
  <div class="rounded-lg border border-primary/30 bg-primary/5 p-3" @click.stop>
    <div class="flex items-center gap-2">
      <Sparkles class="size-3.5 text-primary" />
      <span class="text-xs font-medium text-primary">AI 摘要</span>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        class="ml-auto size-6 text-muted-foreground"
        aria-label="关闭摘要"
        @click="emit('dismiss')"
      >
        <X class="size-3.5" />
      </Button>
    </div>

    <div v-if="summary.status === 'loading'" class="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
      <Loader2 class="size-3.5 animate-spin" />
      正在阅读原文…
    </div>
    <p v-else-if="summary.status === 'error'" class="mt-2 text-sm text-destructive">{{ summary.errorMessage }}</p>
    <div v-else class="collect-summary-markdown mt-2 min-w-0 text-sm leading-6">
      <MarkdownRender :content="summary.text" :custom-id="customId" />
    </div>
  </div>
</template>

<style scoped>
.collect-summary-markdown :deep(p + p) {
  margin-top: 0.5rem;
}

.collect-summary-markdown :deep(ul) {
  margin-top: 0.25rem;
  padding-left: 1.25rem;
  list-style: disc;
}

.collect-summary-markdown :deep(li) {
  margin-top: 0.25rem;
}
</style>

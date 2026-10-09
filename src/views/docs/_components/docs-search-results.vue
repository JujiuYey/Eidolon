<script setup lang="ts">
import type { DocsSearchResult } from '@/types/docs';
import { FileText, SearchX } from 'lucide-vue-next';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';

interface Props {
  results: DocsSearchResult[];
  isSearching: boolean;
  keyword: string;
}

interface Emits {
  (e: 'select', path: string): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();
</script>

<template>
  <div class="flex h-full min-h-0 flex-col rounded-xl border bg-card">
    <div class="flex items-center justify-between gap-2 px-4 py-3">
      <h2 class="text-sm font-semibold">
        搜索「{{ keyword }}」
      </h2>
      <span
        v-if="!isSearching"
        class="text-xs text-muted-foreground tabular-nums"
      >
        {{ results.length }} 处命中
      </span>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-1 px-2 pb-2">
        <template v-if="isSearching">
          <Skeleton
            v-for="index of 4"
            :key="index"
            class="mx-2 h-14"
          />
        </template>

        <div
          v-else-if="results.length === 0"
          class="flex flex-col items-center gap-2 px-2 py-8 text-center"
        >
          <SearchX class="h-5 w-5 text-muted-foreground" />
          <p class="text-xs text-muted-foreground">
            没有找到匹配的内容。
          </p>
        </div>

        <button
          v-for="(result, index) of results"
          v-else
          :key="`${result.path}-${result.line_number}-${index}`"
          type="button"
          class="block w-full rounded-lg px-3 py-2 text-left transition-colors hover:bg-muted/60"
          @click="emit('select', result.path)"
        >
          <div class="flex items-center gap-1.5">
            <FileText class="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <span class="truncate text-sm font-medium">{{ result.name }}</span>
            <span class="ml-auto shrink-0 text-[11px] text-muted-foreground tabular-nums">
              第 {{ result.line_number }} 行
            </span>
          </div>
          <p class="mt-0.5 truncate text-xs text-muted-foreground">
            {{ result.path }}
          </p>
          <p class="mt-1 truncate rounded bg-muted/60 px-2 py-1 font-mono text-xs">
            {{ result.line_text }}
          </p>
        </button>
      </div>
    </ScrollArea>
  </div>
</template>

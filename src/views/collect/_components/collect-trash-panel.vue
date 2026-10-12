<script setup lang="ts">
import { ArchiveRestore, Link2, StickyNote, Trash2 } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import type { CollectItem } from '@/types/collect';

import { formatCreatedAt, hostOf, remainingDays, TRASH_RETENTION_DAYS } from '../utils/display';

defineProps<{
  items: CollectItem[];
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: 'restore', item: CollectItem): void;
  (e: 'purge', item: CollectItem): void;
  (e: 'empty'): void;
}>();

function purgeLabel(item: CollectItem): string {
  return `彻底删除（${remainingDays(item.deleted_at)} 天后也会自动清除）`;
}
</script>

<template>
  <div class="rounded-xl border bg-card">
    <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
      <h3 class="text-sm font-semibold">回收站</h3>
      <p class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
        删除的收藏保留 {{ TRASH_RETENTION_DAYS }} 天，之后自动清除
      </p>
      <Button
        type="button"
        variant="outline"
        size="sm"
        class="text-destructive hover:bg-destructive/10 hover:text-destructive"
        :disabled="items.length === 0"
        @click="emit('empty')"
      >
        <Trash2 class="size-3.5" />
        清空回收站
      </Button>
    </div>

    <div v-if="loading" class="space-y-2 p-4" aria-busy="true">
      <Skeleton v-for="index in 3" :key="index" class="h-12 w-full rounded-lg" />
    </div>

    <ul v-else-if="items.length > 0" class="divide-y">
      <li v-for="item of items" :key="item.id" class="flex items-center gap-3 px-4 py-3">
        <div class="flex size-5 shrink-0 items-center justify-center">
          <Link2 v-if="item.kind === 'link'" class="size-4 text-muted-foreground" aria-hidden="true" />
          <StickyNote v-else class="size-4 text-muted-foreground" aria-hidden="true" />
        </div>

        <div class="min-w-0 flex-1">
          <p class="truncate text-sm">
            {{ item.kind === 'link' ? (item.title ?? hostOf(item.url)) : item.content }}
          </p>
          <p class="mt-0.5 text-xs text-muted-foreground">
            删除于 {{ formatCreatedAt(item.deleted_at ?? 0) }} · 剩余 {{ remainingDays(item.deleted_at) }} 天
          </p>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <Button type="button" variant="outline" size="sm" @click="emit('restore', item)">
            <ArchiveRestore class="size-3.5" />
            恢复
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-8 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            :aria-label="purgeLabel(item)"
            :title="purgeLabel(item)"
            @click="emit('purge', item)"
          >
            <Trash2 class="size-4" />
          </Button>
        </div>
      </li>
    </ul>

    <p v-else class="px-4 py-10 text-center text-sm text-muted-foreground">回收站是空的</p>
  </div>
</template>

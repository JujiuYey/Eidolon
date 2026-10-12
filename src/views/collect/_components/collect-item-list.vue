<script setup lang="ts">
import {
  Archive,
  ArchiveRestore,
  Copy,
  Link2,
  MoreHorizontal,
  Pencil,
  RefreshCw,
  Sparkles,
  StickyNote,
  Trash2,
} from 'lucide-vue-next';
import { ref } from 'vue';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { AiSummaryState } from '@/stores/collect';
import type { CollectItem } from '@/types/collect';

import { formatCreatedAt, hostOf } from '../utils/display';
import CollectSummaryCard from './collect-summary-card.vue';

const props = defineProps<{
  items: CollectItem[];
  /** 链接条目的 AI 摘要（内存态） */
  summaries: Record<string, AiSummaryState>;
}>();

const emit = defineEmits<{
  (e: 'open', item: CollectItem): void;
  (e: 'edit', item: CollectItem): void;
  (e: 'copy', item: CollectItem): void;
  (e: 'refresh', item: CollectItem): void;
  (e: 'summarize', item: CollectItem): void;
  (e: 'dismiss-summary', item: CollectItem): void;
  (e: 'archive', item: CollectItem): void;
  (e: 'remove', item: CollectItem): void;
}>();

/** favicon 加载失败的地址集合：回退为通用图标，避免反复请求 */
const failedFavicons = ref(new Set<string>());

function onFaviconError(url: string) {
  failedFavicons.value.add(url);
}

function archiveLabel(item: CollectItem): string {
  return item.archived ? '取消归档' : '归档';
}

function summaryFor(item: CollectItem): AiSummaryState | undefined {
  return props.summaries[item.id];
}

/** 链接卡的主操作是在浏览器打开，编辑走菜单；记事卡点击即编辑 */
function handleCardClick(item: CollectItem) {
  if (item.kind === 'link') {
    emit('open', item);
  } else {
    emit('edit', item);
  }
}

function cardTitle(item: CollectItem): string {
  return item.kind === 'link' ? '在浏览器打开' : '点击编辑';
}
</script>

<template>
  <ul class="space-y-2">
    <li v-for="item of props.items" :key="item.id" class="space-y-2">
      <div
        class="group flex cursor-pointer items-start gap-3 rounded-lg border bg-card px-4 py-3 transition-colors hover:border-ring/40 hover:bg-muted/30"
        :title="cardTitle(item)"
        @click="handleCardClick(item)"
      >
        <div class="mt-0.5 flex size-5 shrink-0 items-center justify-center">
          <template v-if="item.kind === 'link'">
            <img
              v-if="item.favicon_url && !failedFavicons.has(item.favicon_url)"
              :src="item.favicon_url"
              class="size-4 rounded-sm"
              alt=""
              loading="lazy"
              @error="onFaviconError(item.favicon_url!)"
            />
            <Link2 v-else class="size-4 text-muted-foreground" aria-hidden="true" />
          </template>
          <StickyNote v-else class="size-4 text-muted-foreground" aria-hidden="true" />
        </div>

        <div class="min-w-0 flex-1">
          <template v-if="item.kind === 'link'">
            <h3 class="truncate text-sm font-medium" :class="item.archived ? 'text-muted-foreground' : ''">
              {{ item.title ?? hostOf(item.url) }}
            </h3>
            <p v-if="item.description" class="mt-0.5 line-clamp-2 text-xs text-muted-foreground">
              {{ item.description }}
            </p>
            <p v-if="item.content" class="mt-0.5 line-clamp-1 text-xs text-foreground/80">{{ item.content }}</p>
          </template>
          <p v-else class="line-clamp-4 text-sm" :class="item.archived ? 'text-muted-foreground' : ''">
            {{ item.content }}
          </p>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <span class="text-xs tabular-nums text-muted-foreground">{{ formatCreatedAt(item.created_at) }}</span>

          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                class="size-7 text-muted-foreground"
                aria-label="收藏操作"
                @click.stop
              >
                <MoreHorizontal class="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" @click.stop>
              <DropdownMenuItem @click="emit('edit', item)">
                <Pencil class="size-3.5" />
                编辑
              </DropdownMenuItem>
              <template v-if="item.kind === 'link'">
                <DropdownMenuItem @click="emit('copy', item)">
                  <Copy class="size-3.5" />
                  复制链接
                </DropdownMenuItem>
                <DropdownMenuItem @click="emit('refresh', item)">
                  <RefreshCw class="size-3.5" />
                  重新获取信息
                </DropdownMenuItem>
                <DropdownMenuItem :disabled="summaryFor(item)?.status === 'loading'" @click="emit('summarize', item)">
                  <Sparkles class="size-3.5" />
                  AI 摘要
                </DropdownMenuItem>
              </template>
              <DropdownMenuItem @click="emit('archive', item)">
                <ArchiveRestore v-if="item.archived" class="size-3.5" />
                <Archive v-else class="size-3.5" />
                {{ archiveLabel(item) }}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem class="text-destructive focus:text-destructive" @click="emit('remove', item)">
                <Trash2 class="size-3.5" />
                删除
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <CollectSummaryCard
        v-if="item.kind === 'link' && summaryFor(item)"
        :summary="summaryFor(item)!"
        :custom-id="`collect-summary-${item.id}`"
        @dismiss="emit('dismiss-summary', item)"
      />
    </li>
  </ul>
</template>

<script setup lang="ts">
import { formatDistanceToNow } from 'date-fns';
import { zhCN } from 'date-fns/locale';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { SentEmail } from '@/types/mail';

interface SentListProps {
  items: SentEmail[];
}

defineProps<SentListProps>();

const selectedId = defineModel<string>('selectedId', { required: false });

function formatSentAt(timestamp: number) {
  if (!timestamp) {
    return '';
  }
  return formatDistanceToNow(new Date(timestamp), { addSuffix: true, locale: zhCN });
}

function previewText(email: SentEmail) {
  return email.body.replace(/\s+/g, ' ').substring(0, 120);
}
</script>

<template>
  <ScrollArea class="min-h-0 flex-1">
    <div class="flex flex-1 flex-col gap-2 px-4 pb-4">
      <TransitionGroup name="list">
        <button
          v-for="item of items"
          :key="item.id"
          :class="cn(
            'flex flex-col items-start gap-1.5 rounded-lg border p-3 text-left text-sm outline-none transition-colors duration-150 hover:border-ring/40 hover:bg-muted/40 focus-visible:ring-[3px] focus-visible:ring-ring/50',
            selectedId === item.id && 'border-primary/40 bg-muted',
          )"
          @click="selectedId = item.id"
        >
          <div class="flex w-full items-center gap-2">
            <span
              v-if="item.status === 'failed'"
              class="size-2 shrink-0 rounded-full bg-destructive"
              aria-hidden="true"
            />
            <span class="min-w-0 flex-1 truncate font-medium">{{ item.subject || '（无主题）' }}</span>
            <span class="shrink-0 text-xs text-muted-foreground">
              {{ formatSentAt(item.sent_at) }}
            </span>
          </div>

          <div class="w-full truncate text-xs text-muted-foreground">
            收件人：{{ item.to_addresses || '-' }}
          </div>

          <div class="line-clamp-2 w-full text-xs text-muted-foreground">
            {{ previewText(item) }}
          </div>

          <Badge
            v-if="item.status === 'failed'"
            variant="destructive"
          >
            发送失败
          </Badge>
        </button>
      </TransitionGroup>

      <div
        v-if="items.length === 0"
        class="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground"
      >
        还没有发送记录
      </div>
    </div>
  </ScrollArea>
</template>

<style scoped>
.list-move {
  transition: transform 200ms ease-out;
}

.list-enter-active {
  transition: opacity 200ms ease-out, transform 200ms ease-out;
}

.list-leave-active {
  transition: opacity 150ms ease-in;
  position: absolute;
}

.list-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.list-leave-to {
  opacity: 0;
}
</style>

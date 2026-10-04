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

const styles = {
  height: 'calc(100vh - 150px)',
};

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
  <ScrollArea class="flex" :style="styles">
    <div class="flex-1 flex flex-col gap-2 p-4 pt-0">
      <TransitionGroup name="list" appear>
        <button
          v-for="item of items"
          :key="item.id"
          :class="cn(
            'flex flex-col items-start gap-2 rounded-lg border p-3 text-left text-sm transition-all hover:bg-accent',
            selectedId === item.id && 'bg-muted',
          )"
          @click="selectedId = item.id"
        >
          <div class="flex w-full flex-col gap-1">
            <div class="flex w-full items-center gap-2">
              <span
                :class="cn(
                  'flex h-2 w-2 shrink-0 rounded-full',
                  item.status === 'sent' ? 'bg-emerald-500' : 'bg-red-500',
                )"
              />
              <div class="min-w-0 flex-1 truncate font-semibold">
                {{ item.subject || '（无主题）' }}
              </div>
              <div
                :class="cn(
                  'shrink-0 text-xs',
                  selectedId === item.id ? 'text-foreground' : 'text-muted-foreground',
                )"
              >
                {{ formatSentAt(item.sent_at) }}
              </div>
            </div>

            <div class="truncate text-xs text-muted-foreground">
              收件人：{{ item.to_addresses || '-' }}
            </div>
          </div>

          <div class="line-clamp-2 text-xs text-muted-foreground">
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
.list-move,
.list-enter-active,
.list-leave-active {
  transition: all 0.5s ease;
}

.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateY(15px);
}

.list-leave-active {
  position: absolute;
}
</style>

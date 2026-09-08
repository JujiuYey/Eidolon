<script setup lang="ts">
import { History, Loader2, RefreshCcw, Trash2, AlertTriangle } from 'lucide-vue-next';
import { computed } from 'vue';
import { toast } from 'vue-sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { ApiClientRequestHistory } from '@/types/api-client';

interface Props {
  histories: ApiClientRequestHistory[];
  isLoading: boolean;
  isClearing: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'restore', id: string): void;
  (e: 'clear'): void;
}>();

const statusLabels: Record<ApiClientRequestHistory['status'], string> = {
  success: '成功',
  http_error: 'HTTP 错误',
  network_error: '网络错误',
  timeout: '超时',
  cancelled: '已取消',
};

const statusVariants: Record<ApiClientRequestHistory['status'], 'default' | 'secondary' | 'destructive' | 'outline'> = {
  success: 'default',
  http_error: 'destructive',
  network_error: 'destructive',
  timeout: 'destructive',
  cancelled: 'outline',
};

const sorted = computed(() => {
  return [...props.histories].sort((a, b) => b.executedAt - a.executedAt);
});

function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return '-';
  }
  return date.toLocaleString();
}

function handleRestore(id: string): void {
  emit('restore', id);
  toast.success('已恢复到编辑器（未自动发送）');
}

function handleClear(): void {
  emit('clear');
}
</script>

<template>
  <div class="flex flex-col gap-3 rounded-lg border bg-card p-4">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <History class="size-4" />
        <h3 class="text-sm font-semibold">
          历史记录
        </h3>
        <span class="text-xs text-muted-foreground">最近 {{ sorted.length }} 条</span>
      </div>

      <Button variant="ghost" size="sm" :disabled="isClearing || sorted.length === 0" @click="handleClear">
        <Loader2 v-if="isClearing" class="size-4 animate-spin" />
        <Trash2 v-else class="size-4" />
        清空
      </Button>
    </div>

    <div v-if="isLoading" class="flex items-center gap-2 text-sm text-muted-foreground">
      <Loader2 class="size-4 animate-spin" />
      加载历史…
    </div>

    <div
      v-else-if="sorted.length === 0"
      class="rounded-md border border-dashed bg-muted/20 px-3 py-6 text-center text-xs text-muted-foreground"
    >
      暂无历史记录
    </div>

    <template v-else>
      <Alert>
        <AlertTriangle class="size-4" />
        <AlertTitle>恢复不自动发送</AlertTitle>
        <AlertDescription>
          从历史恢复只更新编辑器，请重新检查后再发送；敏感变量和认证头需要使用当前环境凭据重新解析。
        </AlertDescription>
      </Alert>

      <ScrollArea class="h-72">
        <ul class="flex flex-col gap-2 pr-3">
          <li
            v-for="entry of sorted"
            :key="entry.id"
            class="rounded-md border bg-background p-3 text-xs"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <Badge :variant="statusVariants[entry.status]">
                  {{ statusLabels[entry.status] }}
                </Badge>
                <span class="font-mono">{{ entry.requestSnapshot.method }} {{ entry.statusCode ?? '-' }}</span>
                <span class="text-muted-foreground">{{ entry.durationMs }} ms</span>
              </div>
              <Button variant="ghost" size="icon-sm" :aria-label="`恢复历史 ${entry.id}`" @click="handleRestore(entry.id)">
                <RefreshCcw class="size-4" />
              </Button>
            </div>

            <div class="mt-1 text-muted-foreground">
              {{ formatTimestamp(entry.executedAt) }}
              <span v-if="entry.environmentName"> · 环境 {{ entry.environmentName }}</span>
            </div>

            <div class="mt-2 break-all font-mono text-[11px] text-muted-foreground">
              {{ entry.requestSnapshot.url || '(空 URL)' }}
            </div>

            <p v-if="entry.responseBodyTruncated" class="mt-1 text-[11px] text-muted-foreground">
              响应超过 1 MiB 已被截断。
            </p>
          </li>
        </ul>
      </ScrollArea>
    </template>
  </div>
</template>

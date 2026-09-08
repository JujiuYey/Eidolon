<script setup lang="ts">
import { Clipboard, Loader2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { ApiClientResponseView } from '@/types/api-client';
import { copyToClipboard } from '@/utils/helpers';

interface Props {
  response: ApiClientResponseView | null;
  isLoading: boolean;
}

const props = defineProps<Props>();

const showFormatted = ref(false);

const statusLabel = computed(() => {
  if (!props.response) {
    return '尚未发送';
  }
  switch (props.response.kind) {
    case 'loading':
      return '请求中…';
    case 'success':
      return props.response.meta ? `${props.response.meta.status}` : '响应成功';
    case 'http_error':
      return props.response.meta ? `${props.response.meta.status}` : 'HTTP 错误';
    case 'network_error':
      return '网络错误';
    case 'timeout':
      return '请求超时';
    case 'cancelled':
      return '已取消';
    case 'oversize':
      return '响应超过上限';
    case 'binary':
      return '二进制响应';
    default:
      return '空闲';
  }
});

const statusVariant = computed<'default' | 'secondary' | 'destructive' | 'outline'>(() => {
  if (!props.response) {
    return 'outline';
  }
  switch (props.response.kind) {
    case 'success':
      return 'default';
    case 'loading':
      return 'secondary';
    case 'http_error':
    case 'network_error':
    case 'timeout':
      return 'destructive';
    case 'cancelled':
      return 'outline';
    default:
      return 'outline';
  }
});

const formattedText = computed(() => {
  if (!props.response) {
    return '';
  }
  if (showFormatted.value && props.response.formattedJson) {
    return props.response.formattedJson;
  }
  return props.response.rawText;
});

async function onCopy(): Promise<void> {
  if (!props.response || !props.response.rawText) {
    return;
  }
  const ok = await copyToClipboard(props.response.rawText);
  if (ok) {
    toast.success('已复制响应体');
  }
}
</script>

<template>
  <div class="flex flex-col gap-3 rounded-lg border bg-card p-4">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h3 class="text-sm font-semibold">
          响应
        </h3>
        <Badge :variant="statusVariant">
          {{ statusLabel }}
        </Badge>
        <span v-if="response?.meta" class="text-xs text-muted-foreground">
          {{ response.meta.durationMs }} ms · {{ response.meta.sizeBytes }} bytes
        </span>
      </div>

      <Button variant="ghost" size="sm" :disabled="!response?.rawText" @click="onCopy">
        <Clipboard class="size-4" />
        复制
      </Button>
    </div>

    <div v-if="isLoading" class="flex items-center gap-2 text-sm text-muted-foreground">
      <Loader2 class="size-4 animate-spin" />
      正在等待响应…
    </div>

    <Alert v-else-if="response?.kind === 'network_error' || response?.kind === 'timeout'" variant="destructive">
      <AlertTitle>
        {{ response.kind === 'timeout' ? '请求超时' : '执行错误' }}
      </AlertTitle>
      <AlertDescription>
        {{ response.errorMessage ?? '请检查网络或目标服务' }}
      </AlertDescription>
    </Alert>

    <Alert v-else-if="response?.kind === 'cancelled'" variant="default">
      <AlertTitle>已取消</AlertTitle>
      <AlertDescription>客户端停止等待。服务端可能已经执行，不视为回滚。</AlertDescription>
    </Alert>

    <Alert v-else-if="response?.kind === 'oversize'" variant="destructive">
      <AlertTitle>响应超过上限</AlertTitle>
      <AlertDescription>已停止继续读取，避免无限制缓冲。</AlertDescription>
    </Alert>

    <Alert v-else-if="response?.kind === 'binary'" variant="default">
      <AlertTitle>二进制响应</AlertTitle>
      <AlertDescription>首版不强制转换为文本。</AlertDescription>
    </Alert>

    <template v-else-if="response && (response.kind === 'success' || response.kind === 'http_error')">
      <Tabs
        :model-value="showFormatted ? 'formatted' : 'raw'"
        class="w-full"
        @update:model-value="value => showFormatted = value === 'formatted'"
      >
        <TabsList>
          <TabsTrigger value="raw">
            原文
          </TabsTrigger>
          <TabsTrigger value="formatted" :disabled="!response.formattedJson">
            格式化
          </TabsTrigger>
        </TabsList>
        <TabsContent value="raw" class="mt-0">
          <ScrollArea class="h-64 rounded-md border bg-muted/30 p-3">
            <pre class="whitespace-pre-wrap break-words font-mono text-xs">{{ formattedText }}</pre>
          </ScrollArea>
          <p v-if="response.isJsonTruncated" class="mt-2 text-xs text-muted-foreground">
            历史响应超过 1 MiB 已被截断。
          </p>
        </TabsContent>
        <TabsContent value="formatted" class="mt-0">
          <ScrollArea class="h-64 rounded-md border bg-muted/30 p-3">
            <pre class="whitespace-pre-wrap break-words font-mono text-xs">{{ formattedText }}</pre>
          </ScrollArea>
          <p v-if="!response.formattedJson" class="mt-2 text-xs text-muted-foreground">
            响应不是合法 JSON。
          </p>
        </TabsContent>
      </Tabs>

      <details v-if="response.meta" class="rounded-md border bg-muted/20 p-3">
        <summary class="cursor-pointer text-xs font-medium">
          响应头 ({{ response.meta.headers.length }})
        </summary>
        <ScrollArea class="mt-2 h-32">
          <ul class="space-y-1 text-xs">
            <li v-for="header of response.meta.headers" :key="header.id" class="flex gap-2">
              <span class="font-mono font-medium">{{ header.key || '(未命名)' }}:</span>
              <span class="break-all font-mono">{{ header.value }}</span>
            </li>
          </ul>
        </ScrollArea>
      </details>
    </template>

    <div v-else class="rounded-md border border-dashed bg-muted/20 px-3 py-6 text-center text-xs text-muted-foreground">
      点击发送后将在此显示响应结果。
    </div>
  </div>
</template>

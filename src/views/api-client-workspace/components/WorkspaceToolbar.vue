<script setup lang="ts">
import { ChevronLeft, History, Layers, Network, Sparkles } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';

interface Props {
  projectName: string;
  showAiPanel: boolean;
  showHistoryPanel: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'toggle-ai'): void;
  (e: 'toggle-history'): void;
  (e: 'open-environments'): void;
}>();
</script>

<template>
  <div class="flex shrink-0 items-center justify-between border-b border-border bg-background px-4 py-3">
    <div class="flex items-center gap-2">
      <Button variant="ghost" size="sm" @click="emit('back')">
        <ChevronLeft class="size-4" />
        返回项目
      </Button>
      <Network class="size-5 text-primary" />
      <h1 class="text-base font-semibold">
        {{ projectName || '接口请求工具' }}
      </h1>
    </div>

    <div class="flex items-center gap-2">
      <Button variant="outline" size="sm" @click="emit('open-environments')">
        <Layers class="size-4" />
        环境
      </Button>
      <Button
        :variant="showAiPanel ? 'default' : 'outline'"
        size="sm"
        @click="emit('toggle-ai')"
      >
        <Sparkles class="size-4" />
        AI 面板
      </Button>
      <Button
        :variant="showHistoryPanel ? 'default' : 'outline'"
        size="sm"
        @click="emit('toggle-history')"
      >
        <History class="size-4" />
        历史
      </Button>
    </div>
  </div>
</template>

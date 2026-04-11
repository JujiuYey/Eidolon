<script setup lang="ts">
import { ref } from 'vue';
import type { ModelConfig } from '@/services/model_config';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

interface StaticProviderItem {
  id: string;
  name: string;
  icon: string;
  enabled: boolean;
}

defineProps<{
  modelConfigList: ModelConfig[];
  selectedConfigId: string | null;
}>();

const staticProviders: StaticProviderItem[] = [
  {
    id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    enabled: true,
  },
  {
    id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    enabled: true,
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    enabled: true,
  },
  {
    id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    enabled: true,
  },
];

const localSelectedId = ref('minimax');

function handleSelect(id: string) {
  localSelectedId.value = id;
}
</script>

<template>
  <aside class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10 p-3">
    <nav class="space-y-1.5">
      <button
        v-for="provider of staticProviders"
        :key="provider.id"
        type="button"
        class="flex w-full items-center gap-3 rounded-lg border px-3 py-3 text-left transition-colors"
        :class="localSelectedId === provider.id
          ? 'border-border bg-background shadow-xs'
          : 'border-transparent hover:bg-background/70'"
        @click="handleSelect(provider.id)"
      >
        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-background ring-1 ring-black/5">
          <img
            :src="provider.icon"
            :alt="provider.name"
            class="h-6 w-6 object-contain"
          />
        </div>

        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-foreground">
            {{ provider.name }}
          </p>
        </div>

        <span
          v-if="provider.enabled"
          class="inline-flex shrink-0 items-center rounded-full border border-lime-200 bg-lime-50 px-2 py-0.5 text-[10px] font-medium tracking-[0.08em] text-lime-700"
        >
          ON
        </span>
      </button>
    </nav>
  </aside>
</template>

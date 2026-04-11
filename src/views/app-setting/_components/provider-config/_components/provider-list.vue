<script setup lang="ts">
// import { Badge } from '@/components/ui/badge';
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

interface StaticProviderItem {
  id: string;
  name: string;
  icon: string;
  website?: string;
  apiUrl?: string;
}

const selectedProviderId = defineModel<string | null>('selectedProviderId');

const staticProviders: StaticProviderItem[] = [
  {
    id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    website: 'https://platform.minimaxi.com/',
    apiUrl: 'https://api.minimaxi.com/v1',
  },
  {
    id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    website: 'https://www.volcengine.com/',
    apiUrl: 'https://ark.cn-beijing.volces.com/api/v3',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    website: 'https://www.deepseek.com/',
    apiUrl: 'https://api.deepseek.com',
  },
  {
    id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    website: '127.0.0.1:11434',
    apiUrl: 'http://127.0.0.1:11434/api',
  },
];

function handleSelect(id: string) {
  selectedProviderId.value = id;
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
        :class="selectedProviderId === provider.id
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

        <!--
          <Badge v-if="provider.enabled">
          ON
          </Badge>
        -->
      </button>
    </nav>
  </aside>
</template>

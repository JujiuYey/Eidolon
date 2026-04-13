<script setup lang="ts">
import type { ProviderType } from '@/services/model_config';
import {
  PROVIDER_ICONS,
  PROVIDER_NAMES,
} from '../_shared/provider-icons';

interface StaticProviderItem {
  id: ProviderType;
  name: string;
  icon: string;
  website?: string;
  apiUrl?: string;
}

const selectedProviderId = defineModel<string | null>('selectedProviderId');

const staticProviders: StaticProviderItem[] = [
  {
    id: 'minimax',
    name: PROVIDER_NAMES.minimax,
    icon: PROVIDER_ICONS.minimax,
    website: 'https://platform.minimaxi.com/',
    apiUrl: 'https://api.minimaxi.com/v1',
  },
  {
    id: 'volcengine',
    name: PROVIDER_NAMES.volcengine,
    icon: PROVIDER_ICONS.volcengine,
    website: 'https://www.volcengine.com/',
    apiUrl: 'https://ark.cn-beijing.volces.com/api/v3',
  },
  {
    id: 'deepseek',
    name: PROVIDER_NAMES.deepseek,
    icon: PROVIDER_ICONS.deepseek,
    website: 'https://www.deepseek.com/',
    apiUrl: 'https://api.deepseek.com',
  },
  {
    id: 'ollama',
    name: PROVIDER_NAMES.ollama,
    icon: PROVIDER_ICONS.ollama,
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
          ? 'border-primary bg-primary/50 shadow-xs'
          : 'border-transparent hover:bg-primary/10'"
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
      </button>
    </nav>
  </aside>
</template>

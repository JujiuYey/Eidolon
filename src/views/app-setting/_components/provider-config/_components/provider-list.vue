<script setup lang="ts">
import type { ProviderRegistryItem } from '@/types/provider';

defineProps<{
  providers: ProviderRegistryItem[];
  configuredProviderIds: string[];
}>();

const selectedProviderId = defineModel<string>('selectedProviderId', {
  required: true,
});
</script>

<template>
  <aside class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10 p-3">
    <nav class="space-y-1.5">
      <button
        v-for="provider of providers"
        :key="provider.provider_id"
        type="button"
        class="flex w-full items-center gap-3 rounded-lg border px-3 py-3 text-left transition-colors"
        :class="selectedProviderId === provider.provider_id
          ? 'border-primary bg-primary/20 shadow-xs'
          : 'border-transparent hover:bg-primary/10'"
        @click="selectedProviderId = provider.provider_id"
      >
        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-background ring-1 ring-black/5">
          <img
            v-if="provider.icon"
            :src="provider.icon"
            :alt="provider.name"
            class="h-6 w-6 object-contain"
          />
          <span
            v-else
            class="text-xs font-semibold text-muted-foreground"
          >
            {{ provider.name.slice(0, 2).toUpperCase() }}
          </span>
        </div>

        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-foreground">
            {{ provider.name }}
          </p>
          <p
            v-if="configuredProviderIds.includes(provider.provider_id)"
            class="truncate text-xs text-emerald-500"
          >
            已配置
          </p>
          <p
            v-else
            class="truncate text-xs text-muted-foreground"
          >
            未配置
          </p>
        </div>
      </button>
    </nav>
  </aside>
</template>

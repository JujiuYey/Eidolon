<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { findProviderConfig, PROVIDER_REGISTRY } from '@/config/provider-registry';
import { listProviderConfigs } from '@/services/provider_config';
import type { ProviderConfig } from '@/types/provider';
import ProviderList from './_components/provider-list.vue';
import ProviderConfigPanel from './_components/provider-config-panel.vue';

const providers = ref<ProviderConfig[]>([]);
const selectedProviderId = ref(PROVIDER_REGISTRY[0]?.provider_id ?? '');

const selectedProvider = computed(() =>
  providers.value.find(provider => provider.provider_id === selectedProviderId.value) ?? null,
);

function syncSelection() {
  if (providers.value.length === 0) {
    selectedProviderId.value = '';
    return;
  }

  if (!providers.value.some(provider => provider.provider_id === selectedProviderId.value)) {
    selectedProviderId.value = providers.value[0]!.provider_id;
  }
}

async function loadConfigs() {
  const persistedConfigs = await listProviderConfigs();
  const persistedMap = new Map(persistedConfigs.map(config => [config.provider_id, config]));

  const builtinProviders = PROVIDER_REGISTRY.map(provider => {
    const persisted = persistedMap.get(provider.provider_id);

    return {
      ...provider,
      enabled: persisted?.enabled ?? provider.enabled,
      api_key: persisted?.api_key ?? provider.api_key,
      base_url: persisted?.base_url || provider.base_url,
      is_configured: Boolean(persisted),
    };
  });

  const customProviders: ProviderConfig[] = persistedConfigs
    .filter(config => !findProviderConfig(config.provider_id))
    .map(config => ({
      provider_id: config.provider_id,
      name: config.provider_id,
      icon: undefined,
      website: undefined,
      api_type: 'openai-compatible',
      default_base_url: config.base_url,
      models: [],
      enabled: config.enabled,
      api_key: config.api_key,
      base_url: config.base_url,
      is_builtin: false,
      is_configured: true,
    }));

  providers.value = [...builtinProviders, ...customProviders];
  syncSelection();
}

async function handleSaved(providerId: string) {
  selectedProviderId.value = providerId;
  await loadConfigs();
}

async function handleRemoved() {
  await loadConfigs();
}

onMounted(() => {
  void loadConfigs();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ProviderList
        v-model:selected-provider-id="selectedProviderId"
        :providers="providers"
      />
      <ProviderConfigPanel
        v-if="selectedProvider"
        :selected-provider="selectedProvider"
        @saved="handleSaved"
        @removed="handleRemoved"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
import {
  listProviderModels,
  listProviderSettings,
} from '@/services/provider_config';
import type {
  ProviderModel,
  ProviderRegistryItem,
  ProviderSetting,
} from '@/types/provider';
import ProviderList from './_components/provider-list.vue';
import ProviderConfigPanel from './_components/provider-config-panel.vue';

const providerSettings = ref<ProviderSetting[]>([]);
const providerModels = ref<ProviderModel[]>([]);
const selectedProviderId = ref(PROVIDER_REGISTRY[0]?.provider_id ?? '');

const configuredProviderIds = computed(() =>
  providerSettings.value.map(setting => setting.provider_id),
);

const selectedProvider = computed<ProviderRegistryItem | null>(() =>
  PROVIDER_REGISTRY.find(provider => provider.provider_id === selectedProviderId.value) ?? null,
);

const selectedProviderSetting = computed(() =>
  providerSettings.value.find(setting => setting.provider_id === selectedProviderId.value) ?? null,
);

const selectedProviderModels = computed(() =>
  providerModels.value.filter(model => model.provider_id === selectedProviderId.value),
);

function syncSelection() {
  if (PROVIDER_REGISTRY.length === 0) {
    selectedProviderId.value = '';
    return;
  }

  if (!PROVIDER_REGISTRY.some(provider => provider.provider_id === selectedProviderId.value)) {
    selectedProviderId.value = PROVIDER_REGISTRY[0]!.provider_id;
  }
}

async function loadData() {
  const [settings, models] = await Promise.all([
    listProviderSettings(),
    listProviderModels(),
  ]);

  providerSettings.value = settings;
  providerModels.value = models;
  syncSelection();
}

async function handleSaved() {
  await loadData();
}

async function handleRemoved() {
  await loadData();
}

onMounted(() => {
  void loadData();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ProviderList
        v-model:selected-provider-id="selectedProviderId"
        :providers="PROVIDER_REGISTRY"
        :configured-provider-ids="configuredProviderIds"
      />
      <ProviderConfigPanel
        v-if="selectedProvider"
        :provider="selectedProvider"
        :setting="selectedProviderSetting"
        :models="selectedProviderModels"
        @saved="handleSaved"
        @removed="handleRemoved"
      />
    </div>
  </div>
</template>

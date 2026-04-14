import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { PROVIDER_REGISTRY, findProviderMeta } from '@/config/provider-registry';
import {
  deleteProviderConfig,
  fetchProviderModels,
  listProviderConfigs,
  upsertProviderConfig,
} from '@/services/provider_config';
import type { ProviderConfig } from '@/services/provider_config';

export interface ProviderView {
  id: string;
  name: string;
  icon?: string;
  website?: string;
  defaultBaseUrl: string;
  config: ProviderConfig | null;
  isBuiltin: boolean;
}

export const useProviderStore = defineStore('provider', () => {
  const configs = ref<ProviderConfig[]>([]);
  const isLoading = ref(false);
  const selectedProviderId = ref<string>(PROVIDER_REGISTRY[0]?.id ?? '');
  const modelListCache = ref<Record<string, string[]>>({});
  const modelListLoading = ref<Record<string, boolean>>({});

  const providerViews = computed<ProviderView[]>(() => {
    const configMap = new Map(configs.value.map(config => [config.provider_id, config]));

    const builtinViews = PROVIDER_REGISTRY.map(meta => ({
      id: meta.id,
      name: meta.name,
      icon: meta.icon,
      website: meta.website,
      defaultBaseUrl: meta.defaultBaseUrl,
      config: configMap.get(meta.id) ?? null,
      isBuiltin: true,
    }));

    const customViews = configs.value
      .filter(config => !findProviderMeta(config.provider_id))
      .map(config => ({
        id: config.provider_id,
        name: config.provider_id,
        icon: undefined,
        website: undefined,
        defaultBaseUrl: config.base_url,
        config,
        isBuiltin: false,
      }));

    return [...builtinViews, ...customViews];
  });

  const selectedView = computed(() =>
    providerViews.value.find(view => view.id === selectedProviderId.value) ?? null,
  );

  function syncSelection() {
    if (providerViews.value.length === 0) {
      selectedProviderId.value = '';
      return;
    }

    if (!providerViews.value.some(view => view.id === selectedProviderId.value)) {
      selectedProviderId.value = providerViews.value[0]!.id;
    }
  }

  async function loadConfigs() {
    isLoading.value = true;

    try {
      configs.value = await listProviderConfigs();
      syncSelection();
    } finally {
      isLoading.value = false;
    }
  }

  async function saveConfig(config: ProviderConfig) {
    await upsertProviderConfig(config);
    selectedProviderId.value = config.provider_id;
    await loadConfigs();
  }

  async function removeConfig(providerId: string) {
    await deleteProviderConfig(providerId);
    delete modelListCache.value[providerId];
    delete modelListLoading.value[providerId];
    await loadConfigs();
  }

  async function refreshModelList(
    providerId: string,
    overrides?: {
      baseUrl?: string;
      apiKey?: string;
    },
  ) {
    const view = providerViews.value.find(item => item.id === providerId);
    if (!view) {
      return;
    }

    const meta = findProviderMeta(providerId);
    const baseUrl = overrides?.baseUrl?.trim() || view.config?.base_url || view.defaultBaseUrl;
    const apiKey = overrides?.apiKey ?? view.config?.api_key ?? '';
    const apiType = meta?.apiType ?? 'openai-compatible';

    if (!baseUrl) {
      throw new Error('请先填写 API 地址');
    }

    modelListLoading.value = {
      ...modelListLoading.value,
      [providerId]: true,
    };

    try {
      const models = await fetchProviderModels({
        baseUrl,
        apiKey,
        apiType,
      });

      modelListCache.value = {
        ...modelListCache.value,
        [providerId]: models,
      };
    } finally {
      modelListLoading.value = {
        ...modelListLoading.value,
        [providerId]: false,
      };
    }
  }

  function getModelList(providerId: string): string[] {
    const fetchedModels = modelListCache.value[providerId];
    if (fetchedModels) {
      return fetchedModels;
    }

    return findProviderMeta(providerId)?.defaultModels.map(model => model.id) ?? [];
  }

  return {
    configs,
    isLoading,
    selectedProviderId,
    providerViews,
    selectedView,
    modelListCache,
    modelListLoading,
    loadConfigs,
    saveConfig,
    removeConfig,
    refreshModelList,
    getModelList,
  };
});

<script setup lang="ts">
import {
  Check,
  ExternalLink,
  Eye,
  EyeOff,
  RefreshCw,
  RotateCcw,
  Trash2,
} from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { useProviderStore } from '@/stores/provider';
import type { ProviderConfig } from '@/services/provider_config';
import { getErrorMessage } from '@/utils/helpers';

const store = useProviderStore();
const { modelListLoading, selectedProviderId, selectedView } = storeToRefs(store);

const showApiKey = ref(false);
const deleteConfirmOpen = ref(false);

function buildFormData(): ProviderConfig {
  const view = selectedView.value;
  if (!view) {
    return {
      provider_id: '',
      enabled: true,
      api_key: '',
      base_url: '',
    };
  }

  if (view.config) {
    return { ...view.config };
  }

  return {
    provider_id: view.id,
    enabled: true,
    api_key: '',
    base_url: view.defaultBaseUrl,
  };
}

const formData = ref<ProviderConfig>(buildFormData());

watch(selectedView, () => {
  formData.value = buildFormData();
  showApiKey.value = false;
}, { immediate: true });

const isConfigured = computed(() => Boolean(selectedView.value?.config));
const defaultBaseUrl = computed(() => selectedView.value?.defaultBaseUrl ?? '');
const canRestoreBaseUrl = computed(() =>
  Boolean(defaultBaseUrl.value && formData.value.base_url !== defaultBaseUrl.value),
);
const modelList = computed(() => store.getModelList(selectedProviderId.value));
const isModelListLoading = computed(() => modelListLoading.value[selectedProviderId.value] ?? false);
const isFormValid = computed(() => Boolean(formData.value.base_url.trim()));
const providerInitials = computed(() =>
  (selectedView.value?.name ?? '?').slice(0, 2).toUpperCase(),
);

function restoreBaseUrl() {
  formData.value.base_url = defaultBaseUrl.value;
}

async function saveConfig() {
  try {
    await store.saveConfig({ ...formData.value });
    toast.success('保存成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

function openDeleteConfirm() {
  if (!isConfigured.value) {
    return;
  }

  deleteConfirmOpen.value = true;
}

async function confirmDelete() {
  try {
    await store.removeConfig(selectedProviderId.value);
    deleteConfirmOpen.value = false;
    toast.success('删除成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

async function refreshModels() {
  try {
    await store.refreshModelList(selectedProviderId.value, {
      baseUrl: formData.value.base_url,
      apiKey: formData.value.api_key,
    });
    toast.success('模型列表已更新');
  } catch (error) {
    toast.error(getErrorMessage(error, '拉取模型列表失败'));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <!-- 顶部标题栏 -->
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-3">
        <div class="flex items-center gap-3">
          <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-white shadow-xs ring-1 ring-black/5">
            <img
              v-if="selectedView?.icon"
              :src="selectedView.icon"
              :alt="selectedView.name"
              class="h-7 w-7 object-contain"
            />
            <span
              v-else
              class="text-sm font-semibold"
            >
              {{ providerInitials }}
            </span>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
                {{ selectedView?.name }}
              </h2>
              <Button
                v-if="selectedView?.website"
                as-child
                variant="ghost"
                size="icon-sm"
                class="h-8 w-8 rounded-full text-muted-foreground"
              >
                <a
                  :href="selectedView.website"
                  target="_blank"
                  rel="noreferrer"
                >
                  <ExternalLink class="h-4 w-4" />
                </a>
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-3 pt-1">
        <span class="text-sm font-medium text-muted-foreground">启用</span>
        <Switch v-model="formData.enabled" />
      </div>
    </div>

    <!-- 内容区域 -->
    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-8 px-8 py-6">
        <!-- API 地址 -->
        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 地址</Label>

          <div class="flex flex-col gap-3 md:flex-row">
            <Input
              v-model="formData.base_url"
              type="text"
              placeholder="例如: https://api.openai.com/v1"
              class="h-11 rounded-xl"
            />
            <Button
              type="button"
              variant="outline"
              class="h-11 rounded-xl px-4 text-red-500 hover:text-red-500"
              :disabled="!canRestoreBaseUrl"
              @click="restoreBaseUrl"
            >
              <RotateCcw class="h-4 w-4" />
              重置
            </Button>
          </div>
        </section>

        <!-- API 密钥 -->
        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 密钥</Label>

          <div class="relative">
            <Input
              v-model="formData.api_key"
              :type="showApiKey ? 'text' : 'password'"
              placeholder="请输入 API 密钥"
              class="h-11 rounded-xl pr-11"
            />
            <button
              type="button"
              class="absolute top-1/2 right-3 flex -translate-y-1/2 items-center text-muted-foreground transition hover:text-foreground"
              @click="showApiKey = !showApiKey"
            >
              <Eye v-if="!showApiKey" class="h-4 w-4" />
              <EyeOff v-else class="h-4 w-4" />
            </button>
          </div>
        </section>

        <!-- 模型列表 -->
        <section class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">模型</Label>
              <span class="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {{ modelList.length }}
              </span>
            </div>

            <Button
              type="button"
              variant="outline"
              size="sm"
              class="h-9 gap-2 rounded-xl"
              :disabled="isModelListLoading"
              @click="refreshModels"
            >
              <RefreshCw
                class="h-4 w-4"
                :class="{ 'animate-spin': isModelListLoading }"
              />
              拉取模型列表
            </Button>
          </div>

          <div
            v-if="modelList.length > 0"
            class="space-y-3"
          >
            <div
              v-for="modelId of modelList"
              :key="modelId"
              class="flex w-full items-center justify-between gap-4 rounded-2xl border bg-background px-5 py-4"
            >
              <div class="flex min-w-0 items-center gap-3">
                <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5">
                  <img
                    v-if="selectedView?.icon"
                    :src="selectedView.icon"
                    :alt="selectedView.name"
                    class="h-5 w-5 object-contain"
                  />
                  <span
                    v-else
                    class="text-[10px] font-semibold text-muted-foreground"
                  >
                    {{ providerInitials }}
                  </span>
                </div>

                <span class="truncate text-[15px] font-medium text-foreground">
                  {{ modelId }}
                </span>
              </div>
            </div>
          </div>

          <div
            v-else
            class="rounded-2xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground"
          >
            点击“拉取模型列表”获取该平台支持的模型，或使用默认列表。
          </div>
        </section>
      </div>
    </ScrollArea>

    <!-- 底部操作栏 -->
    <div class="flex flex-wrap items-center justify-end gap-3 border-t bg-background/95 px-8 py-4 backdrop-blur">
      <Button
        type="button"
        variant="destructive"
        class="h-10 rounded-xl"
        :disabled="!isConfigured"
        @click="openDeleteConfirm"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除配置
      </Button>

      <Button
        type="button"
        class="h-10 rounded-xl px-5"
        :disabled="!isFormValid"
        @click="saveConfig"
      >
        <Check class="mr-2 h-4 w-4" />
        保存配置
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除吗？"
    description="删除后无法恢复"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

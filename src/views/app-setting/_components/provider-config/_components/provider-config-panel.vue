<script setup lang="ts">
import {
  Check,
  ExternalLink,
  Eye,
  EyeOff,
  Minus,
  Plus,
  RotateCcw,
  Settings2,
  Trash2,
} from 'lucide-vue-next';
import type { AcceptableValue } from 'reka-ui';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import SagSelect from '@/components/sag/sag-select/index.vue';
import {
  createModelConfig,
  deleteModelConfig,

} from '@/services/model_config';
import type { ModelConfig, ProviderType } from '@/services/model_config';
import { getErrorMessage } from '@/utils/helpers';
import {
  PROVIDER_ICONS,
  PROVIDER_NAMES,
  PROVIDER_FALLBACK_CLASS,
} from '../_shared/provider-icons';

const props = defineProps<{
  selectedProviderId: string;
}>();

const emit = defineEmits<{
  (e: 'refresh'): void;
}>();

// 提供商选项
const providerOptions = [
  { label: 'MiniMax', value: 'minimax' as ProviderType },
  { label: '火山引擎', value: 'volcengine' as ProviderType },
  { label: 'DeepSeek', value: 'deepseek' as ProviderType },
  { label: 'Ollama', value: 'ollama' as ProviderType },
];

function createEmptyFormData(): ModelConfig {
  return {
    provider_type: 'minimax',
    enabled: true,
    api_key: '',
    base_url: '',
    selected_model_id: '',
    catalog: {
      items: [],
    },
    created_at: '',
    updated_at: '',
  };
}

function createProviderFormData(providerType: ProviderType): ModelConfig {
  const defaultUrls: Record<ProviderType, string> = {
    minimax: 'https://api.minimaxi.com/v1',
    volcengine: 'https://ark.cn-beijing.volces.com/api/v3',
    deepseek: 'https://api.deepseek.com/v1',
    ollama: 'http://127.0.0.1:11434/api',
  };

  const defaultModels: Record<ProviderType, string[]> = {
    minimax: ['MiniMax-M2.7', 'MiniMax-M2.7-highspeed', 'MiniMax-Text-01'],
    volcengine: ['doubao-pro-32k', 'doubao-lite-32k'],
    deepseek: ['deepseek-chat', 'deepseek-reasoner'],
    ollama: ['llama3', 'qwen2', 'mistral'],
  };

  return {
    provider_type: providerType,
    enabled: true,
    api_key: '',
    base_url: defaultUrls[providerType],
    selected_model_id: '',
    catalog: {
      items: defaultModels[providerType].map(id => ({
        id,
        name: id,
        enabled: true,
        capabilities: {
          chat: true,
          vision: false,
          tool_call: true,
          reasoning: id.includes('reasoner'),
          embedding: false,
        },
      })),
    },
    created_at: '',
    updated_at: '',
  };
}

const isProviderPanel = computed(() => Boolean(props.selectedProviderId));
const formData = ref<ModelConfig>(createEmptyFormData());
const deleteConfirmOpen = ref(false);
const showApiKey = ref(false);

// 提供商展示信息
const providerPresentation = computed(() => {
  if (isProviderPanel.value) {
    const providerType = props.selectedProviderId as ProviderType;
    return {
      name: PROVIDER_NAMES[providerType] || PROVIDER_NAMES.minimax,
      icon: PROVIDER_ICONS[providerType] || PROVIDER_ICONS.minimax,
      fallbackClass: PROVIDER_FALLBACK_CLASS[providerType] || PROVIDER_FALLBACK_CLASS.minimax,
    };
  }

  const providerType = formData.value.provider_type;
  return {
    name: PROVIDER_NAMES[providerType] || '模型平台',
    icon: PROVIDER_ICONS[providerType],
    fallbackClass: PROVIDER_FALLBACK_CLASS[providerType] || 'bg-neutral-100 text-neutral-600',
  };
});

const panelTitle = computed(() => {
  if (isProviderPanel.value) {
    return providerPresentation.value.name;
  }
  return PROVIDER_NAMES[formData.value.provider_type] || '模型平台';
});

const availableModels = computed(() => {
  if (isProviderPanel.value) {
    const providerType = props.selectedProviderId as ProviderType;
    const defaultModels: Record<ProviderType, string[]> = {
      minimax: ['MiniMax-M2.7', 'MiniMax-M2.7-highspeed', 'MiniMax-Text-01'],
      volcengine: ['doubao-pro-32k', 'doubao-lite-32k'],
      deepseek: ['deepseek-chat', 'deepseek-reasoner'],
      ollama: ['llama3', 'qwen2', 'mistral'],
    };
    return defaultModels[providerType] || [];
  }

  return formData.value.catalog.items.map(item => item.id);
});

const isFormValid = computed(() => {
  return Boolean(
    formData.value.api_key
    && formData.value.base_url
    && formData.value.selected_model_id,
  );
});

const canRestoreBaseUrl = computed(() => {
  const defaultUrls: Record<ProviderType, string> = {
    minimax: 'https://api.minimaxi.com/v1',
    volcengine: 'https://ark.cn-beijing.volces.com/api/v3',
    deepseek: 'https://api.deepseek.com/v1',
    ollama: 'http://127.0.0.1:11434/api',
  };
  const defaultUrl = defaultUrls[formData.value.provider_type];
  return Boolean(defaultUrl && formData.value.base_url !== defaultUrl);
});

function hydrateForm(id: string | null) {
  if (!id) {
    formData.value = createEmptyFormData();
    return;
  }

  formData.value = createProviderFormData(id as ProviderType);
}

watch(
  () => props.selectedProviderId,
  id => {
    hydrateForm(id);
  },
  { immediate: true },
);

function onProviderChange(value: AcceptableValue) {
  const providerType = value as ProviderType;
  formData.value = createProviderFormData(providerType);
}

function selectModel(modelId: string) {
  formData.value.selected_model_id = modelId;
}

function restorePresetBaseUrl() {
  const defaultUrls: Record<ProviderType, string> = {
    minimax: 'https://api.minimaxi.com/v1',
    volcengine: 'https://ark.cn-beijing.volces.com/api/v3',
    deepseek: 'https://api.deepseek.com/v1',
    ollama: 'http://127.0.0.1:11434/api',
  };
  formData.value.base_url = defaultUrls[formData.value.provider_type];
}

function toggleShowApiKey() {
  showApiKey.value = !showApiKey.value;
}

async function saveConfig() {
  if (isProviderPanel.value) {
    return;
  }

  try {
    await createModelConfig(formData.value);
    toast.success('保存成功');
    emit('refresh');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

function deleteConfig() {
  if (!props.selectedProviderId || isProviderPanel.value) {
    return;
  }

  deleteConfirmOpen.value = true;
}

async function deleteConfirm() {
  if (!props.selectedProviderId || isProviderPanel.value) {
    return;
  }

  try {
    await deleteModelConfig(props.selectedProviderId);
    emit('refresh');
    deleteConfirmOpen.value = false;
    toast.success('删除成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

defineExpose({
  formData,
  resetForm: () => {
    formData.value = createEmptyFormData();
  },
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <!-- 顶部标题栏 -->
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-3">
        <div class="flex items-center gap-3">
          <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-white shadow-xs ring-1 ring-black/5">
            <img
              v-if="providerPresentation.icon"
              :src="providerPresentation.icon"
              :alt="providerPresentation.name"
              class="h-7 w-7 object-contain"
            />
            <span
              v-else
              class="text-sm font-semibold"
              :class="providerPresentation.fallbackClass"
            >
              {{ providerPresentation.name.slice(0, 2) }}
            </span>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
                {{ panelTitle }}
              </h2>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                class="h-8 w-8 rounded-full text-muted-foreground"
              >
                <ExternalLink class="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>

        <div
          v-if="!props.selectedProviderId"
          class="grid gap-3 md:grid-cols-[240px_minmax(0,240px)]"
        >
          <SagSelect
            :model-value="formData.provider_type"
            :options="providerOptions"
            placeholder="请选择平台类型"
            :clearable="false"
            @update:model-value="onProviderChange"
          />
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
              @click="restorePresetBaseUrl"
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
              @click="toggleShowApiKey"
            >
              <Eye
                v-if="!showApiKey"
                class="h-4 w-4"
              />
              <EyeOff
                v-else
                class="h-4 w-4"
              />
            </button>
          </div>
        </section>

        <!-- 模型 -->
        <section class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">模型</Label>
              <span class="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {{ availableModels.length }}
              </span>
            </div>

            <div class="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="icon"
                class="size-10 rounded-xl"
              >
                <Plus class="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div
            v-if="availableModels.length > 0"
            class="space-y-3"
          >
            <button
              v-for="model of availableModels"
              :key="model"
              type="button"
              class="flex w-full items-center justify-between gap-4 rounded-2xl border bg-background px-5 py-4 text-left transition hover:bg-muted/20"
              :class="formData.selected_model_id === model ? 'bg-accent/40' : ''"
              @click="selectModel(model)"
            >
              <div class="flex min-w-0 items-center gap-3">
                <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5">
                  <img
                    v-if="providerPresentation.icon"
                    :src="providerPresentation.icon"
                    :alt="providerPresentation.name"
                    class="h-5 w-5 object-contain"
                  />
                  <span
                    v-else
                    class="text-[10px] font-semibold"
                    :class="providerPresentation.fallbackClass"
                  >
                    {{ providerPresentation.name.slice(0, 2) }}
                  </span>
                </div>

                <span class="truncate text-[15px] font-medium text-foreground">
                  {{ model }}
                </span>
              </div>

              <div class="flex shrink-0 items-center gap-2 text-muted-foreground">
                <Settings2 class="h-4 w-4" />
                <Minus class="h-4 w-4" />
              </div>
            </button>
          </div>

          <div
            v-else
            class="rounded-2xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground"
          >
            选择平台后，这里会展示模型列表样式。
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
        :disabled="!props.selectedProviderId || isProviderPanel"
        @click="deleteConfig"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除配置
      </Button>

      <Button
        type="button"
        class="h-10 rounded-xl px-5"
        :disabled="!isFormValid || isProviderPanel"
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
    @confirm="deleteConfirm"
  />
</template>

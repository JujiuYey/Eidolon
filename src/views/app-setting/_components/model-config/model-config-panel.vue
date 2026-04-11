<script setup lang="ts">
import {
  Check,
  ChevronDown,
  CircleHelp,
  ExternalLink,
  Eye,
  EyeOff,
  Loader2,
  Minus,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  Settings2,
  Sparkles,
  Trash2,
  Wrench,
  PlugZap,
} from 'lucide-vue-next';
import type { AcceptableValue } from 'reka-ui';
import { toast } from 'vue-sonner';
import { Badge } from '@/components/ui/badge';
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
  setDefaultModel,
  testAiConnection,
  updateModelConfig,
} from '@/services/model_config';
import type { ModelConfig } from '@/services/model_config';
import { getErrorMessage } from '@/utils/helpers';
import { AI_PROVIDERS, CUSTOM_VENDOR_ID } from './_shared/const';
import { buildModelGroups, resolveProviderPresentation } from './_shared/provider-presentation';

const props = defineProps<{
  modelConfigList: ModelConfig[];
  selectedConfigId: string | null;
}>();

const emit = defineEmits<{
  (e: 'update:selectedConfigId', value: string | null): void;
  (e: 'refresh'): void;
}>();

function createEmptyFormData(): ModelConfig {
  return {
    id: '',
    name: '',
    api_key: '',
    base_url: '',
    model: '',
    temperature: 0.7,
    top_p: 1.0,
    max_tokens: null,
    top_k: null,
    is_active: true,
    is_default: false,
    created_at: '',
    updated_at: '',
  };
}

function normalizeProviderKey(value = '') {
  return value.toLowerCase().replace(/[^a-z0-9]/g, '');
}

function findPresetVendor(value?: string | null) {
  const normalizedValue = normalizeProviderKey(value ?? '');

  if (!normalizedValue) {
    return undefined;
  }

  return AI_PROVIDERS.find(provider =>
    [
      provider.id,
      provider.name,
      provider.base_url,
    ].some(candidate => normalizeProviderKey(candidate).includes(normalizedValue) || normalizedValue.includes(normalizeProviderKey(candidate))),
  );
}

const isPresetVendor = ref(false);
const modelSelectOptions = ref<Array<{ label: string; value: string }>>([]);
const vendorId = ref<string>(CUSTOM_VENDOR_ID);
const formData = ref<ModelConfig>(createEmptyFormData());
const deleteConfirmOpen = ref(false);
const showApiKey = ref(false);
const testingConnection = ref(false);
const defaultSetting = ref(false);

const vendorOptions = computed(() => {
  const presetOptions = AI_PROVIDERS.map(provider => ({
    label: provider.name,
    value: provider.id,
  }));

  return [
    ...presetOptions,
    {
      label: '自定义',
      value: CUSTOM_VENDOR_ID,
    },
  ];
});

const providerPresentation = computed(() => {
  return resolveProviderPresentation({
    vendorId: vendorId.value,
    name: formData.value.name,
    base_url: formData.value.base_url,
    model: formData.value.model,
  });
});

const panelTitle = computed(() => {
  return providerPresentation.value.name || formData.value.name || '模型平台';
});

const panelSubtitle = computed(() => {
  if (!props.selectedConfigId) {
    return '先把样式搭出来，后续你再把保存与同步逻辑接上。';
  }

  if (formData.value.name && formData.value.name !== providerPresentation.value.name) {
    return formData.value.name;
  }

  return '在这里配置 API Key、API 地址和可用模型。';
});

const isFormValid = computed(() => {
  return Boolean(formData.value.name && formData.value.api_key && formData.value.base_url && formData.value.model);
});

const maxTokensModel = computed({
  get: () => formData.value.max_tokens ?? undefined,
  set: (value: number) => {
    formData.value.max_tokens = value ?? null;
  },
});

const topKModel = computed({
  get: () => formData.value.top_k ?? undefined,
  set: (value: number) => {
    formData.value.top_k = value ?? null;
  },
});

const availableModels = computed(() => {
  const models = new Set<string>();

  modelSelectOptions.value.forEach(option => models.add(option.value));

  if (formData.value.model) {
    models.add(formData.value.model);
  }

  return Array.from(models);
});

const modelGroups = computed(() => buildModelGroups(availableModels.value));

const endpointPreview = computed(() => {
  const baseUrl = formData.value.base_url.trim();

  if (!baseUrl) {
    return '预览：填写 API 地址后将在这里显示完整请求路径';
  }

  return `预览：${baseUrl.replace(/\/$/, '')}/chat/completions`;
});

const canRestoreBaseUrl = computed(() => {
  if (!isPresetVendor.value) {
    return false;
  }

  const vendor = findPresetVendor(vendorId.value);
  return Boolean(vendor && vendor.base_url !== formData.value.base_url);
});

function syncPresetState(config: ModelConfig) {
  const vendor = AI_PROVIDERS.find(provider => {
    const currentBaseUrl = normalizeProviderKey(config.base_url);
    const currentName = normalizeProviderKey(config.name);
    const currentModel = normalizeProviderKey(config.model);

    return [
      provider.id,
      provider.name,
      provider.base_url,
    ].some(candidate => {
      const normalizedCandidate = normalizeProviderKey(candidate);
      return [
        currentBaseUrl,
        currentName,
        currentModel,
      ]
        .filter(Boolean)
        .some(currentValue => currentValue.includes(normalizedCandidate) || normalizedCandidate.includes(currentValue));
    });
  });

  if (!vendor) {
    vendorId.value = CUSTOM_VENDOR_ID;
    isPresetVendor.value = false;
    modelSelectOptions.value = [];
    return;
  }

  vendorId.value = vendor.id;
  isPresetVendor.value = true;
  modelSelectOptions.value = vendor.models.map(model => ({
    label: model,
    value: model,
  }));
}

function hydrateForm(id: string | null) {
  if (!id) {
    formData.value = createEmptyFormData();
    vendorId.value = CUSTOM_VENDOR_ID;
    isPresetVendor.value = false;
    modelSelectOptions.value = [];
    return;
  }

  const currentConfig = props.modelConfigList.find(item => item.id === id);

  if (!currentConfig) {
    return;
  }

  formData.value = { ...currentConfig };
  syncPresetState(currentConfig);
}

watch(
  () => props.selectedConfigId,
  id => {
    hydrateForm(id);
  },
  { immediate: true },
);

watch(
  () => props.modelConfigList,
  () => {
    hydrateForm(props.selectedConfigId);
  },
  { deep: true },
);

function onVendorChange(value: AcceptableValue) {
  const nextVendorId = String(value ?? CUSTOM_VENDOR_ID);
  vendorId.value = nextVendorId;

  const vendor = findPresetVendor(nextVendorId);

  if (!vendor || nextVendorId === CUSTOM_VENDOR_ID) {
    isPresetVendor.value = false;
    modelSelectOptions.value = [];

    if (!props.selectedConfigId) {
      formData.value.base_url = '';
      formData.value.model = '';
    }

    return;
  }

  isPresetVendor.value = true;
  modelSelectOptions.value = vendor.models.map(model => ({
    label: model,
    value: model,
  }));

  formData.value.name = vendor.name;
  formData.value.base_url = vendor.base_url;
  formData.value.model = vendor.models[0] ?? formData.value.model;
}

function toggleShowApiKey() {
  showApiKey.value = !showApiKey.value;
}

function selectModel(model: string) {
  formData.value.model = model;
}

function restorePresetBaseUrl() {
  const vendor = findPresetVendor(vendorId.value);

  if (vendor) {
    formData.value.base_url = vendor.base_url;
  }
}

async function saveConfig() {
  try {
    if (props.selectedConfigId) {
      await updateModelConfig(formData.value);
    } else {
      await createModelConfig(formData.value);
    }

    toast.success('保存成功');
    emit('refresh');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

function deleteConfig() {
  if (!props.selectedConfigId) {
    return;
  }

  deleteConfirmOpen.value = true;
}

async function deleteConfirm() {
  if (!props.selectedConfigId) {
    return;
  }

  try {
    await deleteModelConfig(props.selectedConfigId);
    emit('refresh');
    emit('update:selectedConfigId', null);
    deleteConfirmOpen.value = false;
    toast.success('删除成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

async function testConnection() {
  testingConnection.value = true;

  try {
    await testAiConnection({
      api_key: formData.value.api_key,
      base_url: formData.value.base_url,
      model: formData.value.model,
    });
    toast.success('连接成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '测试连接失败'));
  } finally {
    testingConnection.value = false;
  }
}

async function setDefault() {
  if (!props.selectedConfigId) {
    return;
  }

  defaultSetting.value = true;

  try {
    const currentConfig = props.modelConfigList.find(item => item.id === props.selectedConfigId);

    if (!currentConfig || currentConfig.is_default) {
      return;
    }

    await setDefaultModel(props.selectedConfigId);
    toast.success('设置成功');
    emit('refresh');
  } catch (error) {
    toast.error(getErrorMessage(error, '设置失败'));
  } finally {
    defaultSetting.value = false;
  }
}

function getModelCapsules(model: string) {
  const normalized = model.toLowerCase();

  const capsules = [
    {
      key: 'ability',
      icon: Sparkles,
      class: 'bg-sky-50 text-sky-500',
    },
  ];

  if (!normalized.includes('embedding')) {
    capsules.push({
      key: 'tools',
      icon: Wrench,
      class: 'bg-orange-50 text-orange-500',
    });
  }

  return capsules;
}

defineExpose({
  formData,
  resetForm: () => {
    formData.value = createEmptyFormData();
    vendorId.value = CUSTOM_VENDOR_ID;
    isPresetVendor.value = false;
    modelSelectOptions.value = [];
  },
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-3">
        <div class="flex items-center gap-3">
          <div
            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-white shadow-xs ring-1 ring-black/5"
            :class="!providerPresentation.icon ? providerPresentation.iconFallbackClass : ''"
          >
            <img
              v-if="providerPresentation.icon"
              :src="providerPresentation.icon"
              :alt="providerPresentation.name"
              class="h-7 w-7 object-contain"
            />
            <span
              v-else
              class="text-sm font-semibold"
            >
              {{ providerPresentation.initials }}
            </span>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
                {{ panelTitle }}
              </h2>
              <Badge
                v-if="formData.is_default"
                variant="secondary"
                class="rounded-full px-2.5"
              >
                默认
              </Badge>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                class="h-8 w-8 rounded-full text-muted-foreground"
              >
                <ExternalLink class="h-4 w-4" />
              </Button>
            </div>
            <p class="truncate text-sm text-muted-foreground">
              {{ panelSubtitle }}
            </p>
          </div>
        </div>

        <div
          v-if="!props.selectedConfigId"
          class="grid gap-3 md:grid-cols-[240px_minmax(0,240px)]"
        >
          <SagSelect
            :model-value="vendorId"
            :options="vendorOptions"
            placeholder="请选择平台类型"
            :clearable="false"
            @update:model-value="onVendorChange"
          />
          <Input
            v-model="formData.name"
            type="text"
            placeholder="配置名称"
            class="h-10 rounded-xl"
          />
        </div>
      </div>

      <div class="flex items-center gap-3 pt-1">
        <span class="text-sm font-medium text-muted-foreground">启用</span>
        <Switch v-model="formData.is_active" />
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-8 px-8 py-6">
        <section class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">API 密钥</Label>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              class="h-8 w-8 rounded-full text-muted-foreground"
            >
              <Settings2 class="h-4 w-4" />
            </Button>
          </div>

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

          <div class="flex flex-wrap items-center justify-between gap-3 text-xs">
            <button
              type="button"
              class="text-[#4080ff] transition hover:underline"
            >
              点击这里获取密钥
            </button>
            <span class="text-muted-foreground">多个密钥使用逗号分隔</span>
          </div>
        </section>

        <section class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">API 地址</Label>
              <CircleHelp class="h-4 w-4 text-muted-foreground" />
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              class="h-8 w-8 rounded-full text-muted-foreground"
            >
              <Settings2 class="h-4 w-4" />
            </Button>
          </div>

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

          <p class="text-xs text-muted-foreground">
            {{ endpointPreview }}
          </p>
        </section>

        <section class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">模型</Label>
              <span class="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {{ availableModels.length }}
              </span>
              <Sparkles class="h-4 w-4 text-muted-foreground" />
              <Search class="h-4 w-4 text-muted-foreground" />
            </div>

            <div class="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                class="h-10 rounded-xl px-4"
              >
                <RefreshCw class="h-4 w-4" />
                获取模型列表
              </Button>
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
            v-if="!isPresetVendor"
            class="rounded-2xl border border-dashed bg-muted/10 p-4"
          >
            <div class="space-y-2">
              <Label>自定义模型名称</Label>
              <Input
                v-model="formData.model"
                type="text"
                placeholder="例如: gpt-4o, qwen-max, deepseek-chat"
                class="h-11 rounded-xl"
              />
            </div>
          </div>

          <div
            v-if="modelGroups.length > 0"
            class="space-y-3"
          >
            <div
              v-for="group of modelGroups"
              :key="group.title"
              class="overflow-hidden rounded-2xl border bg-background"
            >
              <div class="flex items-center gap-3 border-b bg-muted/20 px-5 py-3 text-sm font-semibold text-foreground">
                <ChevronDown class="h-4 w-4 text-muted-foreground" />
                <span>{{ group.title }}</span>
              </div>

              <div>
                <button
                  v-for="model of group.models"
                  :key="model"
                  type="button"
                  class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left transition hover:bg-muted/20"
                  :class="formData.model === model ? 'bg-accent/40' : ''"
                  @click="selectModel(model)"
                >
                  <div class="flex min-w-0 items-center gap-3">
                    <div
                      class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5"
                      :class="!providerPresentation.icon ? providerPresentation.iconFallbackClass : ''"
                    >
                      <img
                        v-if="providerPresentation.icon"
                        :src="providerPresentation.icon"
                        :alt="providerPresentation.name"
                        class="h-5 w-5 object-contain"
                      />
                      <span
                        v-else
                        class="text-[10px] font-semibold"
                      >
                        {{ providerPresentation.initials }}
                      </span>
                    </div>

                    <span class="truncate text-[15px] font-medium text-foreground">
                      {{ model }}
                    </span>
                  </div>

                  <div class="flex shrink-0 items-center gap-2 text-muted-foreground">
                    <span
                      v-for="capsule of getModelCapsules(model)"
                      :key="capsule.key"
                      class="inline-flex h-7 w-7 items-center justify-center rounded-full"
                      :class="capsule.class"
                    >
                      <component
                        :is="capsule.icon"
                        class="h-3.5 w-3.5"
                      />
                    </span>
                    <Settings2 class="h-4 w-4" />
                    <Minus class="h-4 w-4" />
                  </div>
                </button>
              </div>
            </div>
          </div>

          <div
            v-else
            class="rounded-2xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground"
          >
            选择平台后，这里会展示模型列表样式。
          </div>
        </section>

        <section class="rounded-2xl border bg-muted/10 p-5">
          <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 class="text-base font-semibold text-foreground">
                高级参数
              </h3>
              <p class="text-sm text-muted-foreground">
                先保留现有字段，布局压成更接近截图的轻量两列样式。
              </p>
            </div>
            <Badge
              variant="outline"
              class="rounded-full"
            >
              样式预留
            </Badge>
          </div>

          <div class="grid gap-4 md:grid-cols-2">
            <div class="space-y-2">
              <Label>Temperature</Label>
              <Input
                v-model.number="formData.temperature"
                type="number"
                min="0"
                max="2"
                step="0.1"
                class="h-11 rounded-xl"
              />
            </div>

            <div class="space-y-2">
              <Label>Top P</Label>
              <Input
                v-model.number="formData.top_p"
                type="number"
                min="0"
                max="1"
                step="0.05"
                class="h-11 rounded-xl"
              />
            </div>

            <div class="space-y-2">
              <Label>Max Tokens</Label>
              <Input
                v-model.number="maxTokensModel"
                type="number"
                min="1"
                placeholder="不限制"
                class="h-11 rounded-xl"
              />
            </div>

            <div class="space-y-2">
              <Label>Top K</Label>
              <Input
                v-model.number="topKModel"
                type="number"
                min="1"
                placeholder="不限制"
                class="h-11 rounded-xl"
              />
            </div>
          </div>
        </section>

        <p class="text-sm text-muted-foreground">
          查看
          <button
            type="button"
            class="px-1 text-[#4080ff] transition hover:underline"
          >
            {{ providerPresentation.name }} 文档
          </button>
          和
          <button
            type="button"
            class="px-1 text-[#4080ff] transition hover:underline"
          >
            模型
          </button>
          获取更多详情
        </p>
      </div>
    </ScrollArea>

    <div class="flex flex-wrap items-center justify-between gap-3 border-t bg-background/95 px-8 py-4 backdrop-blur">
      <Button
        type="button"
        variant="outline"
        class="h-10 rounded-xl"
        :disabled="!isFormValid || testingConnection"
        @click="testConnection"
      >
        <Loader2
          v-if="testingConnection"
          class="mr-2 h-4 w-4 animate-spin"
        />
        <PlugZap
          v-else
          class="mr-2 h-4 w-4"
        />
        {{ testingConnection ? '测试中…' : '测试连接' }}
      </Button>

      <div class="flex flex-wrap gap-3">
        <Button
          type="button"
          variant="outline"
          class="h-10 rounded-xl"
          :disabled="defaultSetting || !props.selectedConfigId || formData.is_default"
          @click="setDefault"
        >
          <Settings2 class="mr-2 h-4 w-4" />
          设为默认
        </Button>

        <Button
          type="button"
          variant="destructive"
          class="h-10 rounded-xl"
          :disabled="!props.selectedConfigId"
          @click="deleteConfig"
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
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除吗？"
    description="删除后无法恢复"
    type="destructive"
    @confirm="deleteConfirm"
  />
</template>

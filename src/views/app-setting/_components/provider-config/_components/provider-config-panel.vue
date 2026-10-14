<script setup lang="ts">
import {
  Check,
  Download,
  ExternalLink,
  Eye,
  EyeOff,
  Loader2,
  PencilLine,
  Plus,
  RotateCcw,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';

import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import {
  deleteProviderModels,
  deleteProviderSetting,
  fetchProviderModels,
  replaceProviderModels,
  upsertProviderSetting,
} from '@/services';
import type {
  FetchedProviderModel,
  ProviderModel,
  ProviderModelCapabilities,
  ProviderRegistryItem,
  ProviderSetting,
} from '@/types/provider';
import { getErrorMessage } from '@/utils/helpers';

const props = defineProps<{
  provider: ProviderRegistryItem;
  setting: ProviderSetting | null;
  models: ProviderModel[];
}>();

const emit = defineEmits<{
  (e: 'saved', providerId: string): void;
  (e: 'removed', providerId: string): void;
}>();

const showApiKey = ref(false);
const deleteConfirmOpen = ref(false);
const modelDialogOpen = ref(false);
const editingModelIndex = ref<number | null>(null);
const isFetchingModels = ref(false);
const fetchDialogOpen = ref(false);
const fetchedModels = ref<FetchedProviderModel[]>([]);
const selectedModelIds = ref<string[]>([]);

const capabilityOptions: Array<{
  key: keyof ProviderModelCapabilities;
  label: string;
  description: string;
}> = [
  { key: 'chat', label: '对话', description: '支持常规文本对话。' },
  { key: 'vision', label: '视觉', description: '支持图片等视觉输入。' },
  { key: 'tool_call', label: '工具调用', description: '支持函数或工具调用。' },
  { key: 'reasoning', label: '推理', description: '适合需要显式推理的模型。' },
  { key: 'embedding', label: 'Embedding', description: '支持向量嵌入能力。' },
];

function createEmptyCapabilities(): ProviderModelCapabilities {
  return {
    chat: false,
    vision: false,
    tool_call: false,
    reasoning: false,
    embedding: false,
  };
}

function cloneModel(model: ProviderModel): ProviderModel {
  return {
    provider_id: model.provider_id,
    model_id: model.model_id,
    capabilities: { ...model.capabilities },
  };
}

function createEmptyModel(): ProviderModel {
  return {
    provider_id: props.provider.provider_id,
    model_id: '',
    capabilities: createEmptyCapabilities(),
  };
}

function buildSettingFormData(): ProviderSetting {
  if (props.setting) {
    return { ...props.setting };
  }

  return {
    provider_id: props.provider.provider_id,
    enabled: true,
    api_key: '',
    base_url: props.provider.default_base_url,
  };
}

const settingForm = ref<ProviderSetting>(buildSettingFormData());
const modelItems = ref<ProviderModel[]>(props.models.map(cloneModel));
const modelDraft = ref<ProviderModel>(createEmptyModel());

watch(
  () => [props.provider, props.setting, props.models],
  () => {
    settingForm.value = buildSettingFormData();
    modelItems.value = props.models.map(cloneModel);
    modelDraft.value = createEmptyModel();
    modelDialogOpen.value = false;
    editingModelIndex.value = null;
    showApiKey.value = false;
    fetchDialogOpen.value = false;
    fetchedModels.value = [];
    selectedModelIds.value = [];
    isFetchingModels.value = false;
  },
  { immediate: true },
);

const isConfigured = computed(() => Boolean(props.setting));
const defaultBaseUrl = computed(() => props.provider.default_base_url);
const canRestoreBaseUrl = computed(() =>
  Boolean(defaultBaseUrl.value && settingForm.value.base_url !== defaultBaseUrl.value),
);
const isFormValid = computed(
  () => Boolean(settingForm.value.base_url.trim()) && modelItems.value.every(model => Boolean(model.model_id.trim())),
);
const canSaveModelDraft = computed(
  () => Boolean(modelDraft.value.model_id.trim()) && Object.values(modelDraft.value.capabilities).some(Boolean),
);
const providerInitials = computed(() => props.provider.name.slice(0, 2).toUpperCase());

/** 已在本地配置中的模型 ID：拉取结果里已存在的默认不勾选，避免重复导入 */
const existingModelIds = computed(() => new Set(modelItems.value.map(model => model.model_id)));
const importableModels = computed(() =>
  fetchedModels.value.filter(model => !existingModelIds.value.has(model.modelId)),
);
const isImportableEmpty = computed(() => importableModels.value.length === 0);
const hasSelection = computed(() => selectedModelIds.value.length > 0);
const allImportableSelected = computed(
  () => importableModels.value.length > 0 && selectedModelIds.value.length === importableModels.value.length,
);

function restoreBaseUrl() {
  settingForm.value.base_url = defaultBaseUrl.value;
}

function openCreateModelDialog() {
  editingModelIndex.value = null;
  modelDraft.value = createEmptyModel();
  modelDialogOpen.value = true;
}

function openEditModelDialog(index: number) {
  editingModelIndex.value = index;
  modelDraft.value = cloneModel(modelItems.value[index]!);
  modelDialogOpen.value = true;
}

function saveModelDraft() {
  const nextModel: ProviderModel = {
    provider_id: props.provider.provider_id,
    model_id: modelDraft.value.model_id.trim(),
    capabilities: { ...modelDraft.value.capabilities },
  };

  if (editingModelIndex.value === null) {
    modelItems.value.push(nextModel);
  } else {
    modelItems.value.splice(editingModelIndex.value, 1, nextModel);
  }

  modelDialogOpen.value = false;
}

function removeModel(index: number) {
  modelItems.value.splice(index, 1);
}

function enabledCapabilityLabels(model: ProviderModel) {
  return capabilityOptions.filter(option => model.capabilities[option.key]).map(option => option.label);
}

async function fetchModelList() {
  isFetchingModels.value = true;
  try {
    const models = await fetchProviderModels({
      provider_id: props.provider.provider_id,
      base_url: settingForm.value.base_url,
      api_key: settingForm.value.api_key,
    });

    fetchedModels.value = models;
    selectedModelIds.value = models.map(model => model.modelId).filter(modelId => !existingModelIds.value.has(modelId));
    fetchDialogOpen.value = true;
  } catch (error) {
    toast.error(getErrorMessage(error, '获取模型列表失败'));
  } finally {
    isFetchingModels.value = false;
  }
}

function toggleModelSelection(modelId: string) {
  selectedModelIds.value = selectedModelIds.value.includes(modelId)
    ? selectedModelIds.value.filter(id => id !== modelId)
    : [...selectedModelIds.value, modelId];
}

function toggleSelectAll() {
  selectedModelIds.value = allImportableSelected.value ? [] : importableModels.value.map(model => model.modelId);
}

/**
 * 导入的模型能力全部留空：远端 /models 不返回能力信息，
 * 由用户在模型卡片上手动标记，避免凭模型名猜测。
 */
function importSelectedModels() {
  const selected = new Set(selectedModelIds.value);
  const added = importableModels.value
    .filter(model => selected.has(model.modelId))
    .map<ProviderModel>(model => ({
      provider_id: props.provider.provider_id,
      model_id: model.modelId,
      capabilities: createEmptyCapabilities(),
    }));

  modelItems.value.push(...added);
  fetchDialogOpen.value = false;
  selectedModelIds.value = [];
  toast.success(`已导入 ${added.length} 个模型，请逐个标记能力后保存`);
}

async function saveConfig() {
  try {
    await upsertProviderSetting({ ...settingForm.value });
    await replaceProviderModels(props.provider.provider_id, modelItems.value);
    emit('saved', props.provider.provider_id);
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
    await deleteProviderSetting(props.provider.provider_id);
    await deleteProviderModels(props.provider.provider_id);
    deleteConfirmOpen.value = false;
    emit('removed', props.provider.provider_id);
    toast.success('删除成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-3">
        <div class="flex items-center gap-3">
          <div
            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-md bg-white shadow-xs ring-1 ring-black/5"
          >
            <img v-if="provider.icon" :src="provider.icon" :alt="provider.name" class="h-7 w-7 object-contain" />
            <span v-else class="text-sm font-semibold">
              {{ providerInitials }}
            </span>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
                {{ provider.name }}
              </h2>
              <Button
                v-if="provider.website"
                as-child
                variant="ghost"
                size="icon-sm"
                class="h-8 w-8 rounded-full text-muted-foreground"
              >
                <a :href="provider.website" target="_blank" rel="noreferrer">
                  <ExternalLink class="h-4 w-4" />
                </a>
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-3 pt-1">
        <span class="text-sm font-medium text-muted-foreground">启用</span>
        <Switch v-model="settingForm.enabled" />
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-8 px-8 py-6">
        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 地址</Label>

          <div class="flex flex-col gap-3 md:flex-row">
            <Input
              v-model="settingForm.base_url"
              type="text"
              placeholder="例如: https://api.openai.com/v1"
              class="h-11 rounded-md"
            />
            <Button
              type="button"
              variant="outline"
              class="h-11 rounded-md px-4 text-red-500 hover:text-red-500"
              :disabled="!canRestoreBaseUrl"
              @click="restoreBaseUrl"
            >
              <RotateCcw class="h-4 w-4" />
              重置
            </Button>
          </div>
        </section>

        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 密钥</Label>

          <div class="relative">
            <Input
              v-model="settingForm.api_key"
              :type="showApiKey ? 'text' : 'password'"
              placeholder="请输入 API 密钥"
              class="h-11 rounded-md pr-11"
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

        <section class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">模型</Label>
              <span
                class="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground"
              >
                {{ modelItems.length }}
              </span>
            </div>

            <div class="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                class="h-9 gap-2 rounded-md"
                :disabled="isFetchingModels"
                @click="fetchModelList"
              >
                <Loader2 v-if="isFetchingModels" class="h-4 w-4 animate-spin" />
                <Download v-else class="h-4 w-4" />
                获取模型列表
              </Button>

              <Button
                type="button"
                variant="outline"
                size="sm"
                class="h-9 gap-2 rounded-md"
                @click="openCreateModelDialog"
              >
                <Plus class="h-4 w-4" />
                新增模型
              </Button>
            </div>
          </div>

          <div v-if="modelItems.length > 0" class="space-y-3">
            <div
              v-for="(model, index) of modelItems"
              :key="`${model.model_id}-${index}`"
              class="space-y-3 rounded-lg border bg-background px-5 py-4"
            >
              <div class="flex items-start justify-between gap-4">
                <div class="min-w-0 flex-1 space-y-3">
                  <div class="flex min-w-0 items-center gap-3">
                    <div
                      class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5"
                    >
                      <img
                        v-if="provider.icon"
                        :src="provider.icon"
                        :alt="provider.name"
                        class="h-5 w-5 object-contain"
                      />
                      <span v-else class="text-[10px] font-semibold text-muted-foreground">
                        {{ providerInitials }}
                      </span>
                    </div>

                    <span class="truncate text-[15px] font-medium text-foreground">
                      {{ model.model_id }}
                    </span>
                  </div>

                  <div class="flex flex-wrap gap-2">
                    <span
                      v-for="label of enabledCapabilityLabels(model)"
                      :key="label"
                      class="inline-flex items-center rounded-full bg-muted px-2.5 py-1 text-xs text-muted-foreground"
                    >
                      {{ label }}
                    </span>
                    <span v-if="enabledCapabilityLabels(model).length === 0" class="text-xs text-muted-foreground">
                      未设置能力
                    </span>
                  </div>
                </div>

                <div class="flex shrink-0 items-center gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    class="h-10 rounded-md px-3"
                    @click="openEditModelDialog(index)"
                  >
                    <PencilLine class="h-4 w-4" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    class="h-10 rounded-md px-3 text-destructive hover:text-destructive"
                    @click="removeModel(index)"
                  >
                    <Trash2 class="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </div>
          </div>

          <div v-else class="rounded-lg border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
            还没有模型。点“获取模型列表”从接口拉取，或点“新增模型”手动填写模型 ID 与能力。
          </div>
        </section>
      </div>
    </ScrollArea>

    <div class="flex flex-wrap items-center justify-end gap-3 border-t bg-background/95 px-8 py-4 backdrop-blur">
      <Button
        type="button"
        variant="destructive"
        class="h-10 rounded-md"
        :disabled="!isConfigured"
        @click="openDeleteConfirm"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除配置
      </Button>

      <Button type="button" class="h-10 rounded-md px-5" :disabled="!isFormValid" @click="saveConfig">
        <Check class="mr-2 h-4 w-4" />
        保存配置
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除吗？"
    description="删除后会同时清空该厂商的模型配置"
    type="destructive"
    @confirm="confirmDelete"
  />

  <Dialog v-model:open="fetchDialogOpen">
    <DialogContent class="flex max-h-[70vh] flex-col overflow-hidden sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>选择要导入的模型</DialogTitle>
        <DialogDescription>
          共拉到 {{ fetchedModels.length }} 个模型，已在列表中的会自动跳过。导入后请逐个标记能力再保存。
        </DialogDescription>
      </DialogHeader>

      <div class="min-h-0 flex-1 overflow-y-auto">
        <p v-if="isImportableEmpty" class="py-8 text-center text-sm text-muted-foreground">
          这些模型都已经在列表中了。
        </p>

        <template v-else>
          <button
            type="button"
            class="w-full rounded-md px-3 py-2 text-left text-sm text-muted-foreground transition-colors hover:bg-muted/60"
            @click="toggleSelectAll"
          >
            {{ allImportableSelected ? '取消全选' : '全选' }}
          </button>

          <div class="mt-1 space-y-1">
            <button
              v-for="model of importableModels"
              :key="model.modelId"
              type="button"
              class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors hover:bg-muted/60"
              @click="toggleModelSelection(model.modelId)"
            >
              <span
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded-xs border"
                :class="
                  selectedModelIds.includes(model.modelId) ? 'border-primary bg-primary text-white' : 'border-border'
                "
              >
                <Check v-if="selectedModelIds.includes(model.modelId)" class="h-3 w-3" />
              </span>
              <span class="truncate font-mono text-sm text-foreground">{{ model.modelId }}</span>
            </button>
          </div>
        </template>
      </div>

      <DialogFooter>
        <Button type="button" variant="outline" @click="fetchDialogOpen = false"> 取消 </Button>
        <Button type="button" :disabled="!hasSelection" @click="importSelectedModels">
          导入 {{ selectedModelIds.length }} 个
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <Dialog v-model:open="modelDialogOpen">
    <DialogContent class="sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>
          {{ editingModelIndex === null ? '新增模型' : '编辑模型' }}
        </DialogTitle>
        <DialogDescription> 维护模型 ID，并设置这个模型支持的能力。 </DialogDescription>
      </DialogHeader>

      <div class="space-y-5">
        <div class="space-y-2">
          <Label class="text-sm font-medium text-foreground">模型 ID</Label>
          <Input v-model="modelDraft.model_id" type="text" placeholder="例如: deepseek-chat" class="h-11 rounded-md" />
        </div>

        <div class="space-y-3">
          <Label class="text-sm font-medium text-foreground">能力</Label>

          <div class="space-y-3 rounded-lg border bg-muted/10 p-4">
            <div
              v-for="capability of capabilityOptions"
              :key="capability.key"
              class="flex items-center justify-between gap-4"
            >
              <div class="space-y-1">
                <p class="text-sm font-medium text-foreground">
                  {{ capability.label }}
                </p>
                <p class="text-xs text-muted-foreground">
                  {{ capability.description }}
                </p>
              </div>

              <Switch v-model="modelDraft.capabilities[capability.key]" />
            </div>
          </div>
        </div>
      </div>

      <DialogFooter>
        <Button type="button" variant="outline" @click="modelDialogOpen = false"> 取消 </Button>
        <Button type="button" :disabled="!canSaveModelDraft" @click="saveModelDraft"> 保存模型 </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

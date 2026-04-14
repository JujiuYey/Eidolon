<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { Info, SlidersHorizontal } from 'lucide-vue-next';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
import {
  listDefaultModelSettings,
  upsertDefaultModelSetting,
} from '@/services/default_model';
import {
  listProviderModels,
  listProviderSettings,
} from '@/services/provider_config';
import type { DefaultModelKey, DefaultModelSetting } from '@/types/default-model';
import type { ProviderModel, ProviderSetting } from '@/types/provider';
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
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { getErrorMessage } from '@/utils/helpers';
import { toast } from 'vue-sonner';

interface DefaultModelCard {
  key: DefaultModelKey;
  title: string;
  description: string;
  showInfo?: boolean;
}

interface ModelOption {
  value: string;
  modelId: string;
  providerId: string;
  providerName: string;
  providerIcon?: string;
}

interface ModelParameterForm {
  temperature: string;
  topP: string;
  maxTokens: string;
  presencePenalty: string;
  frequencyPenalty: string;
}

interface ParameterFieldMeta {
  key: keyof ModelParameterForm;
  label: string;
  type: 'number';
  step?: string;
  min?: string;
  max?: string;
  placeholder: string;
  help: string;
  caution: string;
  spanClass?: string;
}

const cards: DefaultModelCard[] = [
  {
    key: 'assistant',
    title: '默认模型',
    description: '创建新助手时使用的模型，如果助手未设置模型，则优先使用此模型',
  },
  {
    key: 'quick',
    title: '快速模型',
    description: '执行话题命名、搜索关键词提炼等简单任务时使用的模型',
    showInfo: true,
  },
  {
    key: 'translation',
    title: '翻译模型',
    description: '翻译服务使用的模型',
  },
  {
    key: 'embedding',
    title: 'Embedding 模型',
    description: '向量检索、语义召回与知识索引时使用的模型',
  },
];

const providerSettings = ref<ProviderSetting[]>([]);
const providerModels = ref<ProviderModel[]>([]);
const defaultModelSettings = ref<DefaultModelSetting[]>([]);
const isLoading = ref(false);
const parameterDialogOpen = ref(false);
const activeCardKey = ref<DefaultModelCard['key']>('assistant');

const selectedModels = reactive<Record<DefaultModelCard['key'], string>>({
  assistant: '',
  quick: '',
  translation: '',
  embedding: '',
});

const parameterForms = reactive<Record<DefaultModelCard['key'], ModelParameterForm>>({
  assistant: createDefaultParameterForm(),
  quick: createDefaultParameterForm(),
  translation: createDefaultParameterForm(),
  embedding: createDefaultParameterForm(),
});

const parameterDraft = ref<ModelParameterForm>(createDefaultParameterForm());

const parameterFields: ParameterFieldMeta[] = [
  {
    key: 'temperature',
    label: 'Temperature',
    type: 'number',
    step: '0.1',
    min: '0',
    max: '2',
    placeholder: '0.7',
    help: '控制输出的随机性，值越高，结果越发散；值越低，结果越稳定。',
    caution: '建议优先在 0 到 1 之间调整，过高可能导致回答不稳定。',
  },
  {
    key: 'topP',
    label: 'Top P',
    type: 'number',
    step: '0.05',
    min: '0',
    max: '1',
    placeholder: '0.9',
    help: '按累计概率筛选候选 token，只在高概率范围内采样。',
    caution: '通常和 Temperature 选一个作为主要调节项，不建议同时大幅调整。',
  },
  {
    key: 'maxTokens',
    label: 'Max Tokens',
    type: 'number',
    min: '1',
    placeholder: '4096',
    help: '限制单次回复最多生成多少 token。',
    caution: '设置过大可能增加响应时延与费用，也可能被服务端再次截断。',
  },
  {
    key: 'presencePenalty',
    label: 'Presence Penalty',
    type: 'number',
    step: '0.1',
    min: '-2',
    max: '2',
    placeholder: '0',
    help: '鼓励模型引入新内容，减少围绕同一话题打转。',
    caution: '值过高会让回答跳得太快，更适合发散型写作，不适合严谨问答。',
  },
  {
    key: 'frequencyPenalty',
    label: 'Frequency Penalty',
    type: 'number',
    step: '0.1',
    min: '-2',
    max: '2',
    placeholder: '0',
    help: '抑制高频重复用词和句式，让输出更紧凑。',
    caution: '值过高可能让模型刻意回避必要重复，影响表达自然度和准确性。',
    spanClass: 'sm:col-span-2',
  },
];

const providerMetaMap = new Map(
  PROVIDER_REGISTRY.map(provider => [provider.provider_id, provider]),
);

const activeCard = computed(() =>
  cards.find(card => card.key === activeCardKey.value) ?? cards[0],
);

function buildModelOptions(
  settings: ProviderSetting[],
  models: ProviderModel[],
): ModelOption[] {
  const enabledProviderIds = new Set(
    settings
      .filter(setting => setting.enabled)
      .map(setting => setting.provider_id),
  );

  return models
    .filter(model => enabledProviderIds.has(model.provider_id))
    .map(model => {
      const provider = providerMetaMap.get(model.provider_id);

      return {
        value: `${model.provider_id}::${model.model_id}`,
        modelId: model.model_id,
        providerId: model.provider_id,
        providerName: provider?.name ?? model.provider_id,
        providerIcon: provider?.icon,
      };
    });
}

const modelOptions = computed<ModelOption[]>(() => {
  return buildModelOptions(providerSettings.value, providerModels.value);
});

function getSelectedOption(cardKey: DefaultModelCard['key']) {
  return modelOptions.value.find(option => option.value === selectedModels[cardKey]);
}

function createDefaultParameterForm(): ModelParameterForm {
  return {
    temperature: '0.7',
    topP: '0.9',
    maxTokens: '4096',
    presencePenalty: '0',
    frequencyPenalty: '0',
  };
}

function openParameterDialog(cardKey: DefaultModelCard['key']) {
  activeCardKey.value = cardKey;
  parameterDraft.value = { ...parameterForms[cardKey] };
  parameterDialogOpen.value = true;
}

function saveParameters() {
  parameterForms[activeCardKey.value] = { ...parameterDraft.value };
  parameterDialogOpen.value = false;
  void persistCardSetting(activeCardKey.value, '参数已保存').catch(error => {
    toast.error(getErrorMessage(error, '参数保存失败'));
  });
}

function applyDefaultModelState(
  settings: DefaultModelSetting[],
  options: ModelOption[],
) {
  const fallbackValue = options[0]?.value ?? '';

  cards.forEach(card => {
    const savedSetting = settings.find(setting => setting.key === card.key);
    const matchedOption = savedSetting
      ? options.find(option =>
          option.providerId === savedSetting.provider_id
          && option.modelId === savedSetting.model_id,
        )
      : null;

    selectedModels[card.key] = matchedOption?.value ?? fallbackValue;

    parameterForms[card.key] = savedSetting
      ? {
          temperature: savedSetting.temperature,
          topP: savedSetting.top_p,
          maxTokens: savedSetting.max_tokens,
          presencePenalty: savedSetting.presence_penalty,
          frequencyPenalty: savedSetting.frequency_penalty,
        }
      : createDefaultParameterForm();
  });
}

async function persistCardSetting(cardKey: DefaultModelKey, successMessage: string) {
  const selectedOption = modelOptions.value.find(option => option.value === selectedModels[cardKey]);
  if (!selectedOption) {
    return;
  }

  const form = parameterForms[cardKey];
  const setting: DefaultModelSetting = {
    key: cardKey,
    provider_id: selectedOption.providerId,
    model_id: selectedOption.modelId,
    temperature: form.temperature,
    top_p: form.topP,
    max_tokens: form.maxTokens,
    presence_penalty: form.presencePenalty,
    frequency_penalty: form.frequencyPenalty,
  };

  await upsertDefaultModelSetting(setting);
  const nextSettings = defaultModelSettings.value.filter(item => item.key !== cardKey);
  nextSettings.push(setting);
  defaultModelSettings.value = nextSettings;
  toast.success(successMessage);
}

function handleModelSelectionChange(cardKey: DefaultModelKey, value: string) {
  selectedModels[cardKey] = value;
  void persistCardSetting(cardKey, '默认模型已保存').catch(error => {
    toast.error(getErrorMessage(error, '默认模型保存失败'));
  });
}

async function loadModels() {
  isLoading.value = true;

  try {
    const [settings, models, defaults] = await Promise.all([
      listProviderSettings(),
      listProviderModels(),
      listDefaultModelSettings(),
    ]);

    providerSettings.value = settings;
    providerModels.value = models;
    defaultModelSettings.value = defaults;

    applyDefaultModelState(defaults, buildModelOptions(settings, models));
  } finally {
    isLoading.value = false;
  }
}

onMounted(() => {
  void loadModels();
});
</script>

<template>
  <div class="space-y-5">
    <section
      v-for="card of cards"
      :key="card.key"
      class="rounded-[26px] border bg-card px-5 py-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]"
    >
      <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
        <span>{{ card.title }}</span>
        <Info
          v-if="card.showInfo"
          class="h-4 w-4 text-muted-foreground"
        />
      </div>

      <div class="mt-4 flex max-w-[420px] items-center gap-2">
        <Select
          :model-value="selectedModels[card.key]"
          :disabled="isLoading || modelOptions.length === 0"
          @update:model-value="value => handleModelSelectionChange(card.key, String(value ?? ''))"
        >
          <SelectTrigger class="h-12 flex-1 rounded-xl">
            <SelectValue placeholder="暂无可用模型">
              <span
                v-if="getSelectedOption(card.key)"
                class="flex min-w-0 items-center gap-3"
              >
                <span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5">
                  <img
                    v-if="getSelectedOption(card.key)?.providerIcon"
                    :src="getSelectedOption(card.key)?.providerIcon"
                    :alt="getSelectedOption(card.key)?.providerName"
                    class="h-4 w-4 object-contain"
                  />
                </span>

                <span class="min-w-0 truncate text-[15px] text-foreground">
                  {{ getSelectedOption(card.key)?.modelId }}
                  <span class="text-muted-foreground"> | {{ getSelectedOption(card.key)?.providerName }}</span>
                </span>
              </span>
            </SelectValue>
          </SelectTrigger>

          <SelectContent>
            <SelectGroup>
              <SelectItem
                v-for="option of modelOptions"
                :key="option.value"
                :value="option.value"
              >
                <span class="flex min-w-0 items-center gap-3">
                  <span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5">
                    <img
                      v-if="option.providerIcon"
                      :src="option.providerIcon"
                      :alt="option.providerName"
                      class="h-4 w-4 object-contain"
                    />
                  </span>

                  <span class="min-w-0 truncate">
                    {{ option.modelId }}
                    <span class="text-muted-foreground"> | {{ option.providerName }}</span>
                  </span>
                </span>
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>

        <Button
          type="button"
          variant="outline"
          size="icon"
          @click="openParameterDialog(card.key)"
        >
          <SlidersHorizontal class="h-4 w-4" />
        </Button>
      </div>

      <p class="mt-3 text-sm text-muted-foreground">
        {{ card.description }}
      </p>
    </section>
  </div>

  <Dialog v-model:open="parameterDialogOpen">
    <DialogContent class="sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>
          {{ activeCard?.title }}参数设置
        </DialogTitle>
        <DialogDescription>
          为当前默认模型配置采样和输出参数。
        </DialogDescription>
      </DialogHeader>

      <div class="grid gap-4 sm:grid-cols-2">
        <div
          v-for="field of parameterFields"
          :key="field.key"
          class="space-y-2"
          :class="field.spanClass"
        >
          <div class="flex items-center gap-2">
            <Label>{{ field.label }}</Label>

            <TooltipProvider :delay-duration="120">
              <Tooltip>
                <TooltipTrigger as-child>
                  <button
                    type="button"
                    class="inline-flex items-center text-muted-foreground transition hover:text-foreground"
                    :aria-label="`${field.label} 说明`"
                  >
                    <Info class="h-4 w-4" />
                  </button>
                </TooltipTrigger>
                <TooltipContent
                  side="top"
                  align="start"
                  class="max-w-xs space-y-2"
                >
                  <p class="text-sm leading-5 text-primary-foreground">
                    {{ field.help }}
                  </p>
                  <p class="text-xs leading-5 text-primary-foreground/85">
                    注意：{{ field.caution }}
                  </p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>

          <Input
            v-model="parameterDraft[field.key]"
            :type="field.type"
            :step="field.step"
            :min="field.min"
            :max="field.max"
            :placeholder="field.placeholder"
          />
        </div>
      </div>

      <DialogFooter>
        <Button
          type="button"
          variant="outline"
          @click="parameterDialogOpen = false"
        >
          取消
        </Button>
        <Button
          type="button"
          @click="saveParameters"
        >
          保存参数
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

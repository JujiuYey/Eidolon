<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { Bot, Cpu, Settings2, Sparkles, Wrench } from 'lucide-vue-next';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
import { listDefaultModelSettings } from '@/services/default_model';
import { listMcpServices } from '@/services/mcp_service';
import {
  listProviderModels,
  listProviderSettings,
} from '@/services/provider_config';
import type {
  AgentProfile,
  AgentProfileInput,
} from '@/types';
import type { DefaultModelSetting } from '@/types/default-model';
import type {
  McpDiscoveredTool,
  McpService,
} from '@/types/mcp-service';
import type {
  ProviderModel,
  ProviderSetting,
} from '@/types/provider';
import { getErrorMessage } from '@/utils/helpers';
import { toast } from 'vue-sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';

interface AgentToolOption {
  key: string;
  serviceId: string;
  serviceName: string;
  name: string;
  title: string;
  description: string;
}

interface AgentFormState {
  name: string;
  description: string;
  providerId: string;
  modelId: string;
  temperature: string;
  maxTokens: string;
  systemPrompt: string;
  enabledMcpServiceIds: string[];
  enabledToolKeys: string[];
}

const props = defineProps<{
  mode: 'create' | 'edit';
  initialProfile?: AgentProfile | null;
}>();

const emit = defineEmits<{
  (e: 'cancel'): void;
  (e: 'save', value: AgentProfileInput): void;
}>();

const isLoading = ref(false);
const providerSettings = ref<ProviderSetting[]>([]);
const providerModels = ref<ProviderModel[]>([]);
const defaultSettings = ref<DefaultModelSetting[]>([]);
const mcpServices = ref<McpService[]>([]);

const providerMetaMap = new Map(
  PROVIDER_REGISTRY.map(provider => [provider.provider_id, provider]),
);

const form = reactive<AgentFormState>({
  name: '',
  description: '',
  providerId: '',
  modelId: '',
  temperature: '0.7',
  maxTokens: '4096',
  systemPrompt: '',
  enabledMcpServiceIds: [],
  enabledToolKeys: [],
});

const enabledProviderOptions = computed(() => {
  return providerSettings.value
    .filter(setting => setting.enabled)
    .map(setting => {
      const meta = providerMetaMap.get(setting.provider_id);

      return {
        id: setting.provider_id,
        name: meta?.name ?? setting.provider_id,
        icon: meta?.icon,
      };
    });
});

const availableModelOptions = computed(() => {
  return providerModels.value
    .filter(model => model.provider_id === form.providerId)
    .map(model => {
      const provider = providerMetaMap.get(model.provider_id);

      return {
        value: model.model_id,
        modelId: model.model_id,
        providerName: provider?.name ?? model.provider_id,
        providerIcon: provider?.icon,
      };
    });
});

const availableMcpServices = computed(() => {
  return mcpServices.value.filter(service => service.enabled);
});

const availableToolOptions = computed<AgentToolOption[]>(() => {
  return availableMcpServices.value
    .filter(service => form.enabledMcpServiceIds.includes(service.id))
    .flatMap(service => {
      const tools = service.discovery?.tools ?? [];

      return tools
        .filter(tool => tool.enabled)
        .map(tool => buildToolOption(service, tool));
    });
});

const selectedModelLabel = computed(() => {
  return availableModelOptions.value.find(option => option.modelId === form.modelId)?.modelId ?? '';
});

const canSave = computed(() => {
  return Boolean(
    form.name.trim()
    && form.providerId
    && form.modelId
    && form.systemPrompt.trim(),
  );
});

watch(() => props.initialProfile, () => {
  hydrateForm();
}, { immediate: true });

watch(availableToolOptions, tools => {
  const availableKeys = new Set(tools.map(tool => tool.key));
  form.enabledToolKeys = form.enabledToolKeys.filter(key => availableKeys.has(key));
});

watch(() => form.providerId, providerId => {
  if (!providerId) {
    form.modelId = '';
    return;
  }

  if (!availableModelOptions.value.some(option => option.modelId === form.modelId)) {
    form.modelId = availableModelOptions.value[0]?.modelId ?? '';
  }
});

function buildToolOption(service: McpService, tool: McpDiscoveredTool): AgentToolOption {
  return {
    key: `${service.id}:${tool.name}`,
    serviceId: service.id,
    serviceName: service.name,
    name: tool.name,
    title: tool.title || tool.name,
    description: tool.description,
  };
}

function hydrateForm() {
  const profile = props.initialProfile;

  form.name = profile?.name ?? '';
  form.description = profile?.description ?? '';
  form.providerId = profile?.providerId ?? '';
  form.modelId = profile?.modelId ?? '';
  form.temperature = profile?.temperature ?? '0.7';
  form.maxTokens = profile?.maxTokens ?? '4096';
  form.systemPrompt = profile?.systemPrompt ?? '';
  form.enabledMcpServiceIds = [...(profile?.enabledMcpServiceIds ?? [])];
  form.enabledToolKeys = [...(profile?.enabledToolKeys ?? [])];
}

function applyDefaultModelSetting() {
  if (props.initialProfile) {
    return;
  }

  const assistantSetting = defaultSettings.value.find(setting => setting.key === 'assistant');
  if (!assistantSetting) {
    return;
  }

  form.providerId = assistantSetting.provider_id;
  form.modelId = assistantSetting.model_id;
  form.temperature = assistantSetting.temperature || '0.7';
  form.maxTokens = assistantSetting.max_tokens || '4096';
}

function toggleMcpService(serviceId: string, enabled: boolean) {
  if (enabled) {
    if (!form.enabledMcpServiceIds.includes(serviceId)) {
      form.enabledMcpServiceIds = [...form.enabledMcpServiceIds, serviceId];
    }
    return;
  }

  form.enabledMcpServiceIds = form.enabledMcpServiceIds.filter(id => id !== serviceId);
  form.enabledToolKeys = form.enabledToolKeys.filter(key => !key.startsWith(`${serviceId}:`));
}

function toggleTool(toolKey: string, enabled: boolean) {
  if (enabled) {
    if (!form.enabledToolKeys.includes(toolKey)) {
      form.enabledToolKeys = [...form.enabledToolKeys, toolKey];
    }
    return;
  }

  form.enabledToolKeys = form.enabledToolKeys.filter(key => key !== toolKey);
}

function handleSave() {
  emit('save', {
    id: props.initialProfile?.id,
    name: form.name.trim(),
    description: form.description.trim(),
    providerId: form.providerId,
    modelId: form.modelId,
    temperature: form.temperature.trim(),
    maxTokens: form.maxTokens.trim(),
    systemPrompt: form.systemPrompt.trim(),
    enabledMcpServiceIds: [...form.enabledMcpServiceIds],
    enabledToolKeys: [...form.enabledToolKeys],
  });
}

async function loadDependencies() {
  isLoading.value = true;

  try {
    const [settings, models, defaults, services] = await Promise.all([
      listProviderSettings(),
      listProviderModels(),
      listDefaultModelSettings(),
      listMcpServices(),
    ]);

    providerSettings.value = settings;
    providerModels.value = models;
    defaultSettings.value = defaults;
    mcpServices.value = services;

    if (!props.initialProfile) {
      applyDefaultModelSetting();
    }

    if (!form.providerId && enabledProviderOptions.value.length > 0) {
      form.providerId = enabledProviderOptions.value[0]!.id;
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '加载 Agent 配置依赖失败'));
  } finally {
    isLoading.value = false;
  }
}

onMounted(() => {
  void loadDependencies();
});
</script>

<template>
  <div class="mx-auto flex h-full max-w-5xl flex-col overflow-hidden">
    <div class="flex shrink-0 flex-col gap-3 border-b pb-5 lg:flex-row lg:items-end lg:justify-between">
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          {{ mode === 'create' ? '新建 Agent' : '编辑 Agent' }}
        </h1>
        <p class="mt-2 text-sm text-muted-foreground">
          先定义模型、提示词和可用能力，后面就可以基于这个 Agent 开始独立对话。
        </p>
      </div>

      <div class="flex flex-wrap gap-2">
        <Button variant="outline" @click="emit('cancel')">
          取消
        </Button>
        <Button :disabled="!canSave || isLoading" @click="handleSave">
          保存并进入对话
        </Button>
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1 pr-2">
      <div class="space-y-6 py-6">
        <section class="rounded-xl border bg-card p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Bot class="size-4 text-primary" />
            <span>基本信息</span>
          </div>

          <div class="mt-5 grid gap-5">
            <div class="space-y-2">
              <Label>名称</Label>
              <Input v-model="form.name" placeholder="例如：Obsidian 助手" />
            </div>

            <div class="space-y-2">
              <Label>描述</Label>
              <Textarea
                v-model="form.description"
                placeholder="简短说明这个 Agent 的职责和风格"
                class="min-h-24"
              />
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Cpu class="size-4 text-primary" />
            <span>模型</span>
          </div>

          <div class="mt-5 grid gap-5 lg:grid-cols-2">
            <div class="space-y-2">
              <Label>Provider</Label>
              <Select v-model="form.providerId" :disabled="enabledProviderOptions.length === 0">
                <SelectTrigger>
                  <SelectValue placeholder="选择模型厂商" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem
                      v-for="provider of enabledProviderOptions"
                      :key="provider.id"
                      :value="provider.id"
                    >
                      {{ provider.name }}
                    </SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>

            <div class="space-y-2">
              <Label>模型</Label>
              <Select v-model="form.modelId" :disabled="availableModelOptions.length === 0">
                <SelectTrigger>
                  <SelectValue placeholder="选择模型">
                    <span v-if="selectedModelLabel" class="truncate">
                      {{ selectedModelLabel }}
                    </span>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem
                      v-for="option of availableModelOptions"
                      :key="option.value"
                      :value="option.modelId"
                    >
                      {{ option.modelId }}
                    </SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>

            <div class="space-y-2">
              <Label>Temperature</Label>
              <Input v-model="form.temperature" placeholder="0.7" />
            </div>

            <div class="space-y-2">
              <Label>Max Tokens</Label>
              <Input v-model="form.maxTokens" placeholder="4096" />
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Sparkles class="size-4 text-primary" />
            <span>提示词</span>
          </div>

          <div class="mt-5 space-y-2">
            <Label>System Prompt</Label>
            <Textarea
              v-model="form.systemPrompt"
              placeholder="描述这个 Agent 的角色、边界和工作方式"
              class="min-h-[220px]"
            />
          </div>
        </section>

        <section class="rounded-xl border bg-card p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Settings2 class="size-4 text-primary" />
            <span>MCP 服务</span>
          </div>

          <div class="mt-5">
            <Alert v-if="availableMcpServices.length === 0">
              <Wrench class="size-4" />
              <AlertTitle>还没有可用 MCP 服务</AlertTitle>
              <AlertDescription>
                先去“应用设置 → MCP 服务”里配置并测试连接，完成后这里就可以直接选择。
              </AlertDescription>
            </Alert>

            <div v-else class="space-y-3">
              <button
                v-for="service of availableMcpServices"
                :key="service.id"
                type="button"
                class="flex w-full items-start justify-between gap-4 rounded-xl border px-4 py-4 text-left transition-colors"
                :class="form.enabledMcpServiceIds.includes(service.id)
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:bg-muted/20'"
                @click="toggleMcpService(service.id, !form.enabledMcpServiceIds.includes(service.id))"
              >
                <div class="min-w-0 space-y-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <p class="text-sm font-medium text-foreground">
                      {{ service.name }}
                    </p>
                    <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                      {{ service.transport_type === 'stdio' ? 'STDIO' : 'HTTP' }}
                    </Badge>
                    <Badge
                      v-if="service.discovery?.tools?.length"
                      variant="outline"
                      class="border-border bg-muted text-muted-foreground"
                    >
                      {{ service.discovery?.tools?.length }} 工具
                    </Badge>
                  </div>
                  <p class="text-sm text-muted-foreground">
                    {{ service.description || service.discovery?.server_name || '这个服务还没有额外描述。' }}
                  </p>
                </div>

                <Switch
                  :model-value="form.enabledMcpServiceIds.includes(service.id)"
                  @update:model-value="toggleMcpService(service.id, Boolean($event))"
                />
              </button>
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-5 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Wrench class="size-4 text-primary" />
            <span>工具</span>
          </div>

          <div class="mt-5">
            <Alert v-if="form.enabledMcpServiceIds.length === 0">
              <Wrench class="size-4" />
              <AlertTitle>先选择 MCP 服务</AlertTitle>
              <AlertDescription>
                选中一个或多个 MCP 服务后，这里才会列出对应的可用工具。
              </AlertDescription>
            </Alert>

            <div v-else-if="availableToolOptions.length === 0" class="rounded-xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
              当前已选服务还没有发现可用工具，或者这些工具在 MCP 设置里被全局禁用了。
            </div>

            <div v-else class="space-y-3">
              <div
                v-for="tool of availableToolOptions"
                :key="tool.key"
                class="flex items-start justify-between gap-4 rounded-xl border px-4 py-4"
              >
                <div class="min-w-0 space-y-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <p class="text-sm font-medium text-foreground">
                      {{ tool.title }}
                    </p>
                    <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                      {{ tool.serviceName }}
                    </Badge>
                  </div>
                  <p class="text-sm text-muted-foreground">
                    {{ tool.description || '这个工具没有额外描述。' }}
                  </p>
                </div>

                <Switch
                  :model-value="form.enabledToolKeys.includes(tool.key)"
                  @update:model-value="toggleTool(tool.key, Boolean($event))"
                />
              </div>
            </div>
          </div>
        </section>
      </div>
    </ScrollArea>
  </div>
</template>

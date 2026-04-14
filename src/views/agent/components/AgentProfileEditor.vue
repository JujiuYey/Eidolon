<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import type { Component } from 'vue';
import {
  Bot,
  Brain,
  Cpu,
  Server,
  Settings2,
  Shield,
  Sparkles,
  Wrench,
} from 'lucide-vue-next';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
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
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
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
import type { McpService } from '@/types/mcp-service';
import type {
  ProviderModel,
  ProviderSetting,
} from '@/types/provider';
import { getErrorMessage } from '@/utils/helpers';
import { toast } from 'vue-sonner';

type AgentEditorSectionKey = 'basic' | 'prompt' | 'permissions' | 'skills' | 'mcp' | 'plugins' | 'advanced';

interface AgentEditorMenu {
  title: string;
  key: AgentEditorSectionKey;
  icon: Component;
  description: string;
  dividerAfter?: boolean;
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
}

const props = defineProps<{
  mode: 'create' | 'edit';
  initialProfile?: AgentProfile | null;
}>();

const emit = defineEmits<{
  (e: 'cancel'): void;
  (e: 'save', value: AgentProfileInput): void;
}>();

const menus: AgentEditorMenu[] = [
  {
    title: '基础设置',
    key: 'basic',
    icon: Bot,
    description: '维护 Agent 名称、简介、模型和基础参数。',
  },
  {
    title: '提示词设置',
    key: 'prompt',
    icon: Sparkles,
    description: '定义这个 Agent 的 system prompt 与行为边界。',
    dividerAfter: true,
  },
  {
    title: '权限模式',
    key: 'permissions',
    icon: Shield,
    description: '后续这里会定义确认策略、执行边界和批准模式。',
  },
  {
    title: 'Skill',
    key: 'skills',
    icon: Brain,
    description: '后续这里会管理这个 Agent 可用的 skills。',
  },
  {
    title: 'MCP 服务',
    key: 'mcp',
    icon: Server,
    description: '选择这个 Agent 可使用的 MCP 服务，默认启用其全部已发现工具。',
  },
  {
    title: '插件',
    key: 'plugins',
    icon: Wrench,
    description: '后续这里会接入插件扩展能力。',
  },
  {
    title: '高级设置',
    key: 'advanced',
    icon: Settings2,
    description: '后续这里放运行时和实验性配置。',
  },
];

const activeSection = ref<AgentEditorSectionKey>('basic');
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
});

const activeMenu = computed<AgentEditorMenu>(() =>
  menus.find(menu => menu.key === activeSection.value) ?? menus[0]!,
);

const enabledProviderOptions = computed(() => {
  return providerSettings.value
    .filter(setting => setting.enabled)
    .map(setting => {
      const meta = providerMetaMap.get(setting.provider_id);

      return {
        id: setting.provider_id,
        name: meta?.name ?? setting.provider_id,
      };
    });
});

const availableModelOptions = computed(() => {
  return providerModels.value
    .filter(model => model.provider_id === form.providerId)
    .map(model => ({
      modelId: model.model_id,
    }));
});

const availableMcpServices = computed(() =>
  mcpServices.value.filter(service => service.enabled),
);

const selectedModelLabel = computed(() => {
  return availableModelOptions.value.find(option => option.modelId === form.modelId)?.modelId ?? '';
});

const selectedMcpServices = computed(() =>
  availableMcpServices.value.filter(service => form.enabledMcpServiceIds.includes(service.id)),
);

const selectedToolCount = computed(() => deriveEnabledToolKeys(form.enabledMcpServiceIds).length);

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

watch(() => form.providerId, providerId => {
  if (!providerId) {
    form.modelId = '';
    return;
  }

  if (!availableModelOptions.value.some(option => option.modelId === form.modelId)) {
    form.modelId = availableModelOptions.value[0]?.modelId ?? '';
  }
});

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

function toggleMcpService(serviceId: string) {
  if (form.enabledMcpServiceIds.includes(serviceId)) {
    form.enabledMcpServiceIds = form.enabledMcpServiceIds.filter(id => id !== serviceId);
    return;
  }

  form.enabledMcpServiceIds = [...form.enabledMcpServiceIds, serviceId];
}

function deriveEnabledToolKeys(serviceIds: string[]) {
  return availableMcpServices.value
    .filter(service => serviceIds.includes(service.id))
    .flatMap(service =>
      (service.discovery?.tools ?? [])
        .filter(tool => tool.enabled)
        .map(tool => `${service.id}:${tool.name}`),
    );
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
    enabledToolKeys: deriveEnabledToolKeys(form.enabledMcpServiceIds),
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
  <div class="flex h-full overflow-hidden">
    <aside class="flex w-56 flex-col border-r bg-sidebar py-4">
      <div class="px-3 py-2">
        <h2 class="mb-2 px-2 text-sm font-semibold text-sidebar-foreground/70">
          {{ mode === 'create' ? '新建 Agent' : '编辑 Agent' }}
        </h2>

        <nav class="space-y-1">
          <template v-for="(item, index) of menus" :key="item.key">
            <Button
              :variant="activeSection === item.key ? 'outline' : 'ghost'"
              class="w-full justify-start gap-3"
              @click="activeSection = item.key"
            >
              <component :is="item.icon" class="h-4 w-4 shrink-0" />
              <span>{{ item.title }}</span>
              <Badge
                v-if="item.key === 'mcp' && form.enabledMcpServiceIds.length > 0"
                variant="outline"
                class="ml-auto border-border bg-muted text-muted-foreground"
              >
                {{ form.enabledMcpServiceIds.length }}
              </Badge>
            </Button>

            <Separator
              v-if="item.dividerAfter && index < menus.length - 1"
              class="my-2"
            />
          </template>
        </nav>
      </div>
    </aside>

    <main class="mx-auto flex max-w-7xl flex-1 flex-col overflow-hidden p-6">
      <div class="mb-6 flex shrink-0 flex-col gap-4 border-b pb-5 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h1 class="text-2xl font-semibold tracking-tight text-foreground">
            {{ activeMenu.title }}
          </h1>
          <p class="mt-2 text-sm text-muted-foreground">
            {{ activeMenu.description }}
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
        <div class="space-y-6 pb-6">
          <template v-if="activeSection === 'basic'">
            <section class="rounded-xl border bg-card p-5 shadow-sm">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Bot class="size-4 text-primary" />
                <span>Agent 信息</span>
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
                <span>模型设置</span>
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
                          :key="option.modelId"
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
          </template>

          <section v-else-if="activeSection === 'prompt'" class="rounded-xl border bg-card p-5 shadow-sm">
            <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
              <Sparkles class="size-4 text-primary" />
              <span>System Prompt</span>
            </div>

            <div class="mt-5 space-y-2">
              <Label>提示词</Label>
              <Textarea
                v-model="form.systemPrompt"
                placeholder="描述这个 Agent 的角色、边界和工作方式"
                class="min-h-[260px]"
              />
            </div>
          </section>

          <template v-else-if="activeSection === 'mcp'">
            <section class="rounded-xl border bg-card p-5 shadow-sm">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Server class="size-4 text-primary" />
                <span>可用 MCP 服务</span>
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
                    @click="toggleMcpService(service.id)"
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

                    <Badge
                      variant="outline"
                      :class="form.enabledMcpServiceIds.includes(service.id)
                        ? 'border-primary/20 bg-primary/10 text-primary'
                        : 'border-border bg-muted text-muted-foreground'"
                    >
                      {{ form.enabledMcpServiceIds.includes(service.id) ? '已启用' : '点击启用' }}
                    </Badge>
                  </button>
                </div>
              </div>
            </section>

            <Alert v-if="form.enabledMcpServiceIds.length > 0">
              <Wrench class="size-4" />
              <AlertTitle>默认启用所选 MCP 的全部工具</AlertTitle>
              <AlertDescription>
                当前已选择 {{ selectedMcpServices.length }} 个 MCP 服务，共会默认启用 {{ selectedToolCount }} 个已发现且全局启用的工具。第一版不再单独勾选工具。
              </AlertDescription>
            </Alert>
          </template>

          <template v-else-if="activeSection === 'permissions'">
            <section class="rounded-xl border border-dashed bg-muted/10 p-6">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Shield class="size-4 text-primary" />
                <span>权限模式</span>
              </div>
              <p class="mt-3 text-sm leading-6 text-muted-foreground">
                这里后续会接入工具执行确认、自动批准策略、可访问范围和风险级别控制。现在先保留占位。
              </p>
            </section>
          </template>

          <template v-else-if="activeSection === 'skills'">
            <section class="rounded-xl border border-dashed bg-muted/10 p-6">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Brain class="size-4 text-primary" />
                <span>Skill</span>
              </div>
              <p class="mt-3 text-sm leading-6 text-muted-foreground">
                这里后续会让你为 Agent 指定默认 skills、偏好的工作流以及可启用的行为模板。现在先保留占位。
              </p>
            </section>
          </template>

          <template v-else-if="activeSection === 'plugins'">
            <section class="rounded-xl border border-dashed bg-muted/10 p-6">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Wrench class="size-4 text-primary" />
                <span>插件</span>
              </div>
              <p class="mt-3 text-sm leading-6 text-muted-foreground">
                这里后续会接入插件能力，比如额外的数据源、动作执行器或扩展面板。现在先保留占位。
              </p>
            </section>
          </template>

          <template v-else-if="activeSection === 'advanced'">
            <section class="rounded-xl border border-dashed bg-muted/10 p-6">
              <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
                <Settings2 class="size-4 text-primary" />
                <span>高级设置</span>
              </div>
              <p class="mt-3 text-sm leading-6 text-muted-foreground">
                这里后续会放运行时实验开关、上下文窗口策略、保留策略等高级选项。现在先保留占位。
              </p>
            </section>
          </template>
        </div>
      </ScrollArea>
    </main>
  </div>
</template>

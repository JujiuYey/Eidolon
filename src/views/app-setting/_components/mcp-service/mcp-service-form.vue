<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ArrowLeft, CircleHelp, FlaskConical, Loader2, Save, Trash2 } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group';
import { Label } from '@/components/ui/label';
import { NativeSelect } from '@/components/ui/native-select';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import {
  deleteMcpService,
  discoverMcpService,
  upsertMcpService,
} from '@/services/mcp_service';
import type {
  McpDiscoveredResource,
  McpDiscoveredResourceTemplate,
  McpDiscoveredTool,
  McpService,
  McpServiceDiscovery,
  McpTransportType,
} from '@/types/mcp-service';
import { getErrorMessage } from '@/utils/helpers';
import type { McpServiceFormMode } from './types';

interface McpServiceFormState {
  id: string;
  name: string;
  description: string;
  transportType: McpTransportType;
  command: string;
  args: string;
  env: string;
  url: string;
  longRunning: boolean;
  timeout: string;
  enabled: boolean;
  discovery: McpServiceDiscovery | null;
}

const props = defineProps<{
  mode: McpServiceFormMode;
  service?: McpService | null;
}>();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'saved', serviceId: string): void;
  (e: 'removed', serviceId: string): void;
}>();

const form = ref<McpServiceFormState>(createFormState(props.mode, props.service));
const activeTab = ref<'general' | 'tools' | 'prompts' | 'resources'>('general');
const isSaving = ref(false);
const isTesting = ref(false);
const deleteConfirmOpen = ref(false);
const argsPlaceholder = 'arg1\narg2';
const envPlaceholder = 'KEY1=value1\nKEY2=value2';
const transportOptions: Array<{ value: McpTransportType; label: string }> = [
  { value: 'stdio', label: '标准输入 / 输出 (stdio)' },
  { value: 'streamable_http', label: 'Streamable HTTP' },
];

const formTitle = computed(() => {
  if (!form.value.id) {
    return '新增 MCP 服务';
  }

  return form.value.name || 'MCP 服务';
});

const showDeleteAction = computed(() => Boolean(form.value.id));
const hasDiscovery = computed(() => Boolean(form.value.discovery));
const toolItems = computed(() => form.value.discovery?.tools ?? []);
const promptItems = computed(() => form.value.discovery?.prompts ?? []);
const resourceItems = computed(() => form.value.discovery?.resources ?? []);
const resourceTemplateItems = computed(() => form.value.discovery?.resource_templates ?? []);
const isStdioTransport = computed(() => form.value.transportType === 'stdio');
const canSave = computed(() => {
  if (!form.value.name.trim()) {
    return false;
  }

  if (form.value.transportType === 'stdio') {
    return Boolean(form.value.command.trim());
  }

  return Boolean(form.value.url.trim());
});
const canTest = computed(() => canSave.value && !isTesting.value);
const lastTestedLabel = computed(() => {
  if (!form.value.discovery?.tested_at) {
    return '';
  }

  return new Date(form.value.discovery.tested_at).toLocaleString('zh-CN');
});

watch(
  () => [props.mode, props.service?.id],
  () => {
    form.value = createFormState(props.mode, props.service);
    activeTab.value = 'general';
  },
  { immediate: true },
);

function createFormState(mode: McpServiceFormMode, service?: McpService | null): McpServiceFormState {
  return {
    id: service?.id ?? '',
    name: service?.name ?? '',
    description: service?.description ?? '',
    transportType: service?.transport_type ?? 'stdio',
    command: service?.command ?? '',
    args: service?.args ?? '',
    env: service?.env ?? '',
    url: service?.url ?? '',
    longRunning: service?.long_running ?? false,
    timeout: String(service?.timeout_seconds ?? 60),
    enabled: service?.enabled ?? mode !== 'create',
    discovery: service?.discovery ?? null,
  };
}

function buildPayload(): McpService {
  const parsedTimeout = Number.parseInt(form.value.timeout, 10);

  return {
    id: form.value.id,
    name: form.value.name.trim(),
    description: form.value.description.trim(),
    enabled: form.value.enabled,
    transport_type: form.value.transportType,
    command: form.value.command.trim(),
    args: form.value.args,
    env: form.value.env,
    url: form.value.url.trim(),
    long_running: form.value.longRunning,
    timeout_seconds: Number.isFinite(parsedTimeout) && parsedTimeout > 0 ? parsedTimeout : 60,
    discovery: form.value.discovery,
  };
}

function setPreferredDiscoveryTab(discovery: McpServiceDiscovery) {
  if (discovery.tools.length > 0) {
    activeTab.value = 'tools';
    return;
  }

  if (discovery.prompts.length > 0) {
    activeTab.value = 'prompts';
    return;
  }

  if (discovery.resources.length > 0 || discovery.resource_templates.length > 0) {
    activeTab.value = 'resources';
  }
}

async function saveConfig() {
  isSaving.value = true;

  try {
    const payload = buildPayload();
    const serviceId = await upsertMcpService(payload);
    form.value.id = serviceId;
    toast.success('MCP 服务已保存');
    emit('saved', serviceId);
  } catch (error) {
    toast.error(getErrorMessage(error, '保存 MCP 服务失败'));
  } finally {
    isSaving.value = false;
  }
}

async function testConnection() {
  isTesting.value = true;

  try {
    const discovery = await discoverMcpService(buildPayload());
    form.value.discovery = discovery;
    setPreferredDiscoveryTab(discovery);
    toast.success(`连接成功，发现 ${discovery.tools.length} 个工具`);
  } catch (error) {
    toast.error(getErrorMessage(error, '测试 MCP 连接失败'));
  } finally {
    isTesting.value = false;
  }
}

async function confirmDelete() {
  if (!form.value.id) {
    return;
  }

  try {
    const removedId = await deleteMcpService(form.value.id);
    deleteConfirmOpen.value = false;
    toast.success('MCP 服务已删除');
    emit('removed', removedId);
  } catch (error) {
    toast.error(getErrorMessage(error, '删除 MCP 服务失败'));
  }
}

function schemaSummary(tool: McpDiscoveredTool) {
  if (!tool.input_schema || typeof tool.input_schema !== 'object' || Array.isArray(tool.input_schema)) {
    return '参数结构未知';
  }

  const schema = tool.input_schema as {
    properties?: Record<string, unknown>;
    required?: string[];
  };
  const propertyCount = Object.keys(schema.properties ?? {}).length;
  const requiredCount = Array.isArray(schema.required) ? schema.required.length : 0;

  if (propertyCount === 0) {
    return '无需参数';
  }

  return `${propertyCount} 个参数，${requiredCount} 个必填`;
}

function resourceSubtitle(item: McpDiscoveredResource | McpDiscoveredResourceTemplate) {
  return item.description || item.mime_type || '无额外描述';
}
</script>

<template>
  <ScrollArea class="h-full pr-3">
    <section class="space-y-5 pb-6">
      <Button variant="ghost" size="icon" @click="emit('back')">
        <ArrowLeft class="size-4" />
        <span class="sr-only">返回 MCP 服务列表</span>
      </Button>

      <article class="rounded-xl border border-border/70 bg-card px-5 py-5 shadow-sm">
        <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="min-w-0 space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <h1 class="text-[1.35rem] font-semibold tracking-tight">
                {{ formTitle }}
              </h1>
              <Badge v-if="form.transportType === 'stdio'" variant="outline" class="border-primary/20 bg-primary/10 text-primary">
                STDIO
              </Badge>
              <Badge v-else variant="outline" class="border-primary/20 bg-primary/10 text-primary">
                HTTP
              </Badge>
              <Badge v-if="hasDiscovery" variant="outline" class="border-border bg-muted text-muted-foreground">
                {{ toolItems.length }} 工具
              </Badge>
            </div>

            <p class="text-sm text-muted-foreground">
              {{ hasDiscovery && form.discovery?.server_name
                ? `${form.discovery.server_name}${form.discovery.server_version ? ` · ${form.discovery.server_version}` : ''}`
                : '先保存配置，再测试连接并发现工具、提示和资源。' }}
            </p>
            <p v-if="lastTestedLabel" class="text-xs text-muted-foreground">
              最近测试：{{ lastTestedLabel }}
            </p>
          </div>

          <div class="flex flex-wrap items-center justify-end gap-2 self-end sm:self-auto">
            <Switch v-model="form.enabled" />
            <Button variant="outline" size="sm" :disabled="!canTest" @click="testConnection">
              <Loader2 v-if="isTesting" class="size-4 animate-spin" />
              <FlaskConical v-else class="size-4" />
              <span>测试连接</span>
            </Button>
            <Button variant="outline" size="sm" :disabled="!showDeleteAction" @click="deleteConfirmOpen = true">
              <Trash2 class="size-4" />
              <span>删除</span>
            </Button>
            <Button size="sm" :disabled="!canSave || isSaving" @click="saveConfig">
              <Loader2 v-if="isSaving" class="size-4 animate-spin" />
              <Save v-else class="size-4" />
              <span>保存</span>
            </Button>
          </div>
        </div>

        <Separator class="my-5" />

        <Tabs v-model="activeTab" class="space-y-6">
          <TabsList class="grid w-full grid-cols-4">
            <TabsTrigger value="general">
              通用
            </TabsTrigger>
            <TabsTrigger value="tools">
              工具
              <span v-if="toolItems.length > 0" class="text-muted-foreground">({{ toolItems.length }})</span>
            </TabsTrigger>
            <TabsTrigger value="prompts">
              提示
            </TabsTrigger>
            <TabsTrigger value="resources">
              资源
            </TabsTrigger>
          </TabsList>

          <TabsContent value="general" class="mt-0 space-y-6">
            <section v-if="form.discovery?.instructions" class="rounded-xl border bg-muted/20 p-4">
              <p class="text-sm font-medium text-foreground">
                服务说明
              </p>
              <p class="mt-2 text-sm leading-6 text-muted-foreground">
                {{ form.discovery.instructions }}
              </p>
            </section>

            <section class="space-y-2.5">
              <div class="flex items-center gap-1">
                <span class="text-primary">*</span>
                <Label>名称</Label>
              </div>
              <Input
                v-model="form.name"
                placeholder="MCP 服务名称"
                class="h-11"
              />
            </section>

            <section class="space-y-2.5">
              <Label>描述</Label>
              <Textarea
                v-model="form.description"
                placeholder="给这个服务写一段备注，方便后续识别"
                class="min-h-24"
              />
            </section>

            <section class="space-y-2.5">
              <div class="flex items-center gap-1">
                <span class="text-primary">*</span>
                <Label>类型</Label>
              </div>
              <NativeSelect
                v-model="form.transportType"
                class="h-11 w-full"
              >
                <option v-for="option of transportOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </NativeSelect>
            </section>

            <template v-if="isStdioTransport">
              <section class="space-y-2.5">
                <div class="flex items-center gap-1">
                  <span class="text-primary">*</span>
                  <Label>命令</Label>
                </div>
                <Input
                  v-model="form.command"
                  placeholder="例如: npx、uvx、node"
                  class="h-11"
                />
              </section>

              <section class="space-y-2.5">
                <div class="flex items-center gap-1.5">
                  <Label>参数</Label>
                  <CircleHelp class="size-4 text-muted-foreground" />
                </div>
                <Textarea
                  v-model="form.args"
                  :placeholder="argsPlaceholder"
                  class="min-h-[96px]"
                />
              </section>

              <section class="space-y-2.5">
                <div class="flex items-center gap-1.5">
                  <Label>环境变量</Label>
                  <CircleHelp class="size-4 text-muted-foreground" />
                </div>
                <Textarea
                  v-model="form.env"
                  :placeholder="envPlaceholder"
                  class="min-h-[96px]"
                />
              </section>

              <section class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div class="flex items-center gap-1.5">
                  <Label>长时间运行模式</Label>
                  <CircleHelp class="size-4 text-muted-foreground" />
                </div>
                <Switch v-model="form.longRunning" />
              </section>
            </template>

            <template v-else>
              <section class="space-y-2.5">
                <div class="flex items-center gap-1">
                  <span class="text-primary">*</span>
                  <Label>Streamable HTTP 地址</Label>
                </div>
                <Input
                  v-model="form.url"
                  placeholder="例如: http://localhost:3000/mcp"
                  class="h-11"
                />
              </section>
            </template>

            <section class="space-y-2.5">
              <div class="flex items-center gap-1.5">
                <Label>超时</Label>
                <CircleHelp class="size-4 text-muted-foreground" />
              </div>
              <InputGroup>
                <InputGroupInput
                  v-model="form.timeout"
                  class="h-11"
                />
                <InputGroupAddon
                  align="inline-end"
                  class="h-11 px-3 text-sm text-foreground/70"
                >
                  s
                </InputGroupAddon>
              </InputGroup>
            </section>
          </TabsContent>

          <TabsContent value="tools" class="mt-0">
            <div v-if="toolItems.length === 0" class="rounded-xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
              连接成功后，这里会显示 MCP server 暴露的工具。你可以提前在“通用”里测试连接。
            </div>

            <div v-else class="space-y-3">
              <article
                v-for="tool of toolItems"
                :key="tool.name"
                class="rounded-xl border px-4 py-4"
              >
                <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                  <div class="min-w-0 flex-1 space-y-2">
                    <div class="flex flex-wrap items-center gap-2">
                      <h3 class="text-sm font-semibold text-foreground">
                        {{ tool.title || tool.name }}
                      </h3>
                      <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                        {{ schemaSummary(tool) }}
                      </Badge>
                    </div>

                    <p class="text-sm leading-6 text-muted-foreground">
                      {{ tool.description || '这个工具没有提供额外描述。' }}
                    </p>
                  </div>

                  <div class="grid shrink-0 grid-cols-2 gap-6 sm:min-w-[220px]">
                    <div class="space-y-1 text-sm">
                      <p class="font-medium text-foreground">
                        启用工具
                      </p>
                      <p class="text-xs text-muted-foreground">
                        后续对话阶段是否允许使用
                      </p>
                      <Switch v-model="tool.enabled" />
                    </div>

                    <div class="space-y-1 text-sm">
                      <p class="font-medium text-foreground">
                        自动批准
                      </p>
                      <p class="text-xs text-muted-foreground">
                        后续执行时是否跳过人工确认
                      </p>
                      <Switch v-model="tool.auto_approve" />
                    </div>
                  </div>
                </div>
              </article>
            </div>
          </TabsContent>

          <TabsContent value="prompts" class="mt-0">
            <div v-if="promptItems.length === 0" class="rounded-xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
              这个 MCP server 目前没有暴露提示模板，或者还没有完成连接测试。
            </div>

            <div v-else class="space-y-3">
              <article
                v-for="prompt of promptItems"
                :key="prompt.name"
                class="rounded-xl border px-4 py-4"
              >
                <div class="space-y-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <h3 class="text-sm font-semibold text-foreground">
                      {{ prompt.title || prompt.name }}
                    </h3>
                    <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                      {{ prompt.arguments.length }} 参数
                    </Badge>
                  </div>

                  <p class="text-sm leading-6 text-muted-foreground">
                    {{ prompt.description || '这个提示模板没有额外描述。' }}
                  </p>

                  <div v-if="prompt.arguments.length > 0" class="flex flex-wrap gap-2 pt-1">
                    <Badge
                      v-for="argument of prompt.arguments"
                      :key="`${prompt.name}-${argument.name}`"
                      variant="outline"
                      class="border-border bg-background text-muted-foreground"
                    >
                      {{ argument.name }}{{ argument.required ? ' *' : '' }}
                    </Badge>
                  </div>
                </div>
              </article>
            </div>
          </TabsContent>

          <TabsContent value="resources" class="mt-0 space-y-6">
            <section class="space-y-3">
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-semibold text-foreground">
                  资源
                </h3>
                <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                  {{ resourceItems.length }}
                </Badge>
              </div>

              <div v-if="resourceItems.length === 0" class="rounded-xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
                暂无可列举的资源。
              </div>

              <template v-else>
                <article
                  v-for="resource of resourceItems"
                  :key="resource.uri"
                  class="rounded-xl border px-4 py-4"
                >
                  <h4 class="text-sm font-semibold text-foreground">
                    {{ resource.title || resource.name }}
                  </h4>
                  <p class="mt-2 text-sm leading-6 text-muted-foreground">
                    {{ resourceSubtitle(resource) }}
                  </p>
                  <p class="mt-2 break-all text-xs text-muted-foreground">
                    {{ resource.uri }}
                  </p>
                </article>
              </template>
            </section>

            <section class="space-y-3">
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-semibold text-foreground">
                  资源模板
                </h3>
                <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
                  {{ resourceTemplateItems.length }}
                </Badge>
              </div>

              <div v-if="resourceTemplateItems.length === 0" class="rounded-xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground">
                暂无资源模板。
              </div>

              <template v-else>
                <article
                  v-for="resourceTemplate of resourceTemplateItems"
                  :key="resourceTemplate.uri_template"
                  class="rounded-xl border px-4 py-4"
                >
                  <h4 class="text-sm font-semibold text-foreground">
                    {{ resourceTemplate.title || resourceTemplate.name }}
                  </h4>
                  <p class="mt-2 text-sm leading-6 text-muted-foreground">
                    {{ resourceSubtitle(resourceTemplate) }}
                  </p>
                  <p class="mt-2 break-all text-xs text-muted-foreground">
                    {{ resourceTemplate.uri_template }}
                  </p>
                </article>
              </template>
            </section>
          </TabsContent>
        </Tabs>
      </article>
    </section>
  </ScrollArea>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除这个 MCP 服务吗？"
    description="删除后会同时清空这次连接测试发现的工具、提示与资源快照。"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

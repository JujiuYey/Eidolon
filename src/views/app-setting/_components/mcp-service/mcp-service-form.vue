<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ArrowLeft, ChevronDown, CircleHelp, Save, Trash2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group';
import { Label } from '@/components/ui/label';
import { NativeSelect } from '@/components/ui/native-select';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import type { McpServiceCard, McpServiceFormMode } from './types';

interface McpServiceFormState {
  name: string;
  description: string;
  transportType: string;
  command: string;
  args: string;
  env: string;
  longRunning: boolean;
  timeout: string;
  enabled: boolean;
}

const props = defineProps<{
  mode: McpServiceFormMode;
  service?: McpServiceCard | null;
}>();

const emit = defineEmits<{
  (e: 'back'): void;
}>();

const form = ref<McpServiceFormState>(createFormState(props.mode, props.service));
const argsPlaceholder = 'arg1\narg2';
const envPlaceholder = 'KEY1=value1\nKEY2=value2';

const formTitle = computed(() => {
  if (props.mode === 'create') {
    return '新增 MCP 服务';
  }

  return props.service?.name ?? 'MCP 服务';
});

const showDeleteAction = computed(() => props.mode === 'edit');

watch(
  () => [props.mode, props.service?.id],
  () => {
    form.value = createFormState(props.mode, props.service);
  },
  { immediate: true },
);

function createFormState(mode: McpServiceFormMode, service?: McpServiceCard | null): McpServiceFormState {
  return {
    name: mode === 'edit' ? service?.name ?? 'MCP 服务' : '',
    description: '',
    transportType: 'stdio',
    command: '',
    args: '',
    env: '',
    longRunning: false,
    timeout: '60',
    enabled: mode === 'edit' ? Boolean(service?.enabled) : false,
  };
}
</script>

<template>
  <ScrollArea class="h-full pr-3">
    <section class="space-y-5 pb-6">
      <Button
        variant="ghost"
        size="icon"
        class="size-9 rounded-full"
        @click="emit('back')"
      >
        <ArrowLeft class="size-4" />
        <span class="sr-only">返回 MCP 服务列表</span>
      </Button>

      <article class="rounded-[1.75rem] border border-border/70 bg-card px-5 py-5 shadow-sm">
        <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="flex flex-wrap items-center gap-2">
            <h1 class="text-[1.35rem] font-semibold tracking-tight">
              {{ formTitle }}
            </h1>
            <Button
              variant="outline"
              size="sm"
              class="h-8 rounded-lg px-3"
            >
              日志
            </Button>
            <Button
              v-if="showDeleteAction"
              variant="ghost"
              size="icon-sm"
              class="size-8 rounded-full text-destructive/80 hover:bg-destructive/10 hover:text-destructive"
            >
              <Trash2 class="size-4" />
              <span class="sr-only">删除服务</span>
            </Button>
          </div>

          <div class="flex items-center gap-3 self-end sm:self-auto">
            <Switch v-model="form.enabled" />
            <Button
              variant="outline"
              size="sm"
              class="h-10 rounded-full px-4 text-muted-foreground"
              disabled
            >
              <Save class="size-4" />
              <span>保存</span>
            </Button>
          </div>
        </div>

        <Separator class="my-5" />

        <div class="mb-6 border-b">
          <div class="inline-flex border-b-2 border-destructive pb-3 text-base font-medium text-destructive">
            通用
          </div>
        </div>

        <div class="space-y-6">
          <section class="space-y-2.5">
            <div class="flex items-center gap-1">
              <span class="text-destructive">*</span>
              <Label>名称</Label>
            </div>
            <Input
              v-model="form.name"
              placeholder="MCP 服务名称"
              class="h-11 rounded-xl"
            />
          </section>

          <section class="space-y-2.5">
            <Label>描述</Label>
            <Textarea
              v-model="form.description"
              placeholder="描述"
              class="min-h-24 rounded-xl"
            />
          </section>

          <section class="space-y-2.5">
            <div class="flex items-center gap-1">
              <span class="text-destructive">*</span>
              <Label>类型</Label>
            </div>
            <NativeSelect
              v-model="form.transportType"
              class="h-11 w-full rounded-xl"
            >
              <option value="stdio">
                标准输入 / 输出 (stdio)
              </option>
              <option value="sse">
                Server-Sent Events (sse)
              </option>
              <option value="http">
                Streamable HTTP
              </option>
            </NativeSelect>
          </section>

          <section class="space-y-2.5">
            <div class="flex items-center gap-1">
              <span class="text-destructive">*</span>
              <Label>命令</Label>
            </div>
            <Input
              v-model="form.command"
              placeholder="uvx or npx"
              class="h-11 rounded-xl"
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
              class="min-h-[96px] rounded-xl"
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
              class="min-h-[96px] rounded-xl"
            />
          </section>

          <section class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div class="flex items-center gap-1.5">
              <Label>长时间运行模式</Label>
              <CircleHelp class="size-4 text-muted-foreground" />
            </div>
            <Switch v-model="form.longRunning" />
          </section>

          <section class="space-y-2.5">
            <div class="flex items-center gap-1.5">
              <Label>超时</Label>
              <CircleHelp class="size-4 text-muted-foreground" />
            </div>
            <InputGroup class="rounded-xl">
              <InputGroupInput
                v-model="form.timeout"
                class="h-11 rounded-l-xl"
              />
              <InputGroupAddon
                align="inline-end"
                class="h-11 px-3 text-sm text-foreground/70"
              >
                s
              </InputGroupAddon>
            </InputGroup>
          </section>

          <button
            type="button"
            class="inline-flex items-center gap-2 pt-2 text-base font-medium text-destructive transition hover:text-destructive/90"
          >
            <ChevronDown class="size-4" />
            <span>高级设置</span>
          </button>
        </div>
      </article>
    </section>
  </ScrollArea>
</template>

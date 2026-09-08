<script setup lang="ts">
import { ClipboardPaste, Loader2, Save, Send, Square, Sparkles } from 'lucide-vue-next';
import { computed } from 'vue';
import { toast } from 'vue-sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { ApiClientEnvironment, ApiClientHttpMethod, ApiClientRequestBody } from '@/types/api-client';
import { API_CLIENT_HTTP_METHODS } from '@/types/api-client';
import { useApiClientStore } from '@/stores/api-client';
import BodyEditor from './BodyEditor.vue';
import KeyValueEditor from './KeyValueEditor.vue';

interface Props {
  isSending: boolean;
  isSaving: boolean;
  isDirty: boolean;
  draftName: string;
  draftMethod: ApiClientHttpMethod;
  draftUrl: string;
  draftTimeoutMs: number;
  draftQuery: ReturnType<typeof useApiClientStore>['draft'] extends infer R
    ? R extends { query: infer Q }
      ? Q
      : never
    : never;
  draftHeaders: ReturnType<typeof useApiClientStore>['draft'] extends infer R
    ? R extends { headers: infer H }
      ? H
      : never
    : never;
  draftBody: ApiClientRequestBody;
  environments: ApiClientEnvironment[];
  activeEnvironmentId: string | null;
  showAiButton?: boolean;
}

const props = withDefaults(defineProps<Props>(), { showAiButton: true });

const emit = defineEmits<{
  (e: 'update:draftName', value: string): void;
  (e: 'update:draftMethod', value: ApiClientHttpMethod): void;
  (e: 'update:draftUrl', value: string): void;
  (e: 'update:draftTimeoutMs', value: number): void;
  (e: 'update:draftQuery', value: NonNullable<Props['draftQuery']>): void;
  (e: 'update:draftHeaders', value: NonNullable<Props['draftHeaders']>): void;
  (e: 'update:draftBody', value: ApiClientRequestBody): void;
  (e: 'save'): void;
  (e: 'send'): void;
  (e: 'cancel'): void;
  (e: 'pasteUrl'): void;
  (e: 'openAi'): void;
  (e: 'selectEnvironment', value: string): void;
}>();

const store = useApiClientStore();

const hasAuthHeader = computed(() => {
  return props.draftHeaders.some(row => row.enabled && /^(authorization|proxy-authorization|x-api-key|api-key)$/i.test(row.key.trim()));
});

function emitRows(rows: NonNullable<Props['draftQuery']>): void {
  emit('update:draftQuery', rows);
}

function emitHeaderRows(rows: NonNullable<Props['draftHeaders']>): void {
  emit('update:draftHeaders', rows);
}

function onSave(): void {
  if (props.draftName.trim() === '') {
    toast.error('请输入请求名称');
    return;
  }
  emit('save');
}

function onSend(): void {
  if (props.draftName.trim() === '') {
    toast.error('请输入请求名称');
    return;
  }
  emit('send');
}

function onCancel(): void {
  emit('cancel');
}

function onPasteUrl(): void {
  if (!props.draftUrl.trim()) {
    toast.error('请先粘贴 URL');
    return;
  }
  emit('pasteUrl');
}

function onSelectEnvironment(value: string | number | null | undefined): void {
  if (value === null || value === undefined) {
    return;
  }
  const normalized = String(value);
  if (!normalized) {
    return;
  }
  emit('selectEnvironment', normalized);
}

function handleNameInput(value: string | number): void {
  emit('update:draftName', String(value ?? ''));
}

const methodProxy = computed<ApiClientHttpMethod>({
  get: () => props.draftMethod,
  set: value => emit('update:draftMethod', value as ApiClientHttpMethod),
});

const environmentProxy = computed<string>({
  get: () => props.activeEnvironmentId ?? '',
  set: value => onSelectEnvironment(value),
});

function handleUrlInput(value: string | number): void {
  emit('update:draftUrl', String(value ?? ''));
}

function handleTimeoutInput(value: string | number): void {
  emit('update:draftTimeoutMs', Number(value) || 0);
}

const unsavedHint = computed(() => props.isDirty);

void store;
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-3 rounded-lg border bg-card p-4">
      <div class="flex flex-wrap items-center gap-2">
        <Input
          :model-value="draftName"
          placeholder="请求名称"
          class="min-w-[180px] flex-1"
          @update:model-value="handleNameInput"
        />

        <NativeSelect
          v-model="methodProxy"
          class="min-w-[100px]"
        >
          <NativeSelectOption v-for="method of API_CLIENT_HTTP_METHODS" :key="method" :value="method">
            {{ method }}
          </NativeSelectOption>
        </NativeSelect>

        <NativeSelect
          v-model="environmentProxy"
          class="min-w-[140px]"
        >
          <NativeSelectOption value="">
            选择环境
          </NativeSelectOption>
          <NativeSelectOption v-for="env of environments" :key="env.id" :value="env.id">
            {{ env.name }}
          </NativeSelectOption>
        </NativeSelect>

        <Button variant="outline" :disabled="isSaving || isSending" @click="onSave">
          <Loader2 v-if="isSaving" class="size-4 animate-spin" />
          <Save v-else class="size-4" />
          保存
        </Button>

        <Button v-if="!isSending" :disabled="isSaving" @click="onSend">
          <Send class="size-4" />
          发送
        </Button>

        <Button v-else variant="destructive" @click="onCancel">
          <Square class="size-4" />
          取消
        </Button>

        <Button v-if="showAiButton" variant="ghost" @click="emit('openAi')">
          <Sparkles class="size-4" />
          AI 生成
        </Button>
      </div>

      <div class="flex flex-col gap-2">
        <Label>URL</Label>
        <div class="flex gap-2">
          <Input
            :model-value="draftUrl"
            placeholder="https://example.com/path?foo=bar"
            class="flex-1 font-mono"
            @update:model-value="handleUrlInput"
          />
          <Button variant="outline" size="icon" aria-label="粘贴并拆分 URL" @click="onPasteUrl">
            <ClipboardPaste class="size-4" />
          </Button>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <Label class="w-20">超时 (毫秒)</Label>
        <Input
          type="number"
          :model-value="String(draftTimeoutMs)"
          min="0"
          step="100"
          class="w-32"
          @update:model-value="handleTimeoutInput"
        />
      </div>

      <Alert v-if="unsavedHint">
        <AlertTitle>存在未保存修改</AlertTitle>
        <AlertDescription>
          切换请求、项目或关闭页面前，请保存或放弃当前编辑。
        </AlertDescription>
      </Alert>

      <Alert v-if="!hasAuthHeader && (draftMethod === 'POST' || draftMethod === 'PUT' || draftMethod === 'PATCH' || draftMethod === 'DELETE')">
        <AlertTitle>认证提示</AlertTitle>
        <AlertDescription>
          如需鉴权，请使用 Headers 配置 Bearer Token、API Key 等认证。
        </AlertDescription>
      </Alert>
    </div>

    <Tabs default-value="params" class="rounded-lg border bg-card">
      <TabsList class="border-b">
        <TabsTrigger value="params">
          Params
        </TabsTrigger>
        <TabsTrigger value="headers">
          Headers
        </TabsTrigger>
        <TabsTrigger value="body">
          Body
        </TabsTrigger>
      </TabsList>

      <TabsContent value="params" class="p-4">
        <KeyValueEditor
          :model-value="draftQuery"
          add-label="新增参数"
          empty-hint="暂无 Query 参数"
          @update:model-value="emitRows"
        />
      </TabsContent>

      <TabsContent value="headers" class="p-4">
        <KeyValueEditor
          :model-value="draftHeaders"
          add-label="新增 Header"
          empty-hint="暂无 Header"
          @update:model-value="emitHeaderRows"
        />
      </TabsContent>

      <TabsContent value="body" class="p-4">
        <BodyEditor
          :model-value="draftBody"
          @update:model-value="(body: ApiClientRequestBody) => emit('update:draftBody', body)"
        />
      </TabsContent>
    </Tabs>
  </div>
</template>

<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { Tabs, TabsContent, TabsList } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import type { ApiClientBodyKind, ApiClientRequestBody } from '@/types/api-client';
import KeyValueEditor from './KeyValueEditor.vue';

interface Props {
  modelValue: ApiClientRequestBody;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'update:modelValue': [body: ApiClientRequestBody];
}>();

const currentKind = computed<ApiClientBodyKind>(() => props.modelValue.kind);

const jsonValidation = ref<{ ok: boolean; error: string | null }>({ ok: true, error: null });

function validateJson(text: string): { ok: boolean; error: string | null } {
  const trimmed = text.trim();
  if (!trimmed) {
    return { ok: true, error: null };
  }
  try {
    JSON.parse(trimmed);
    return { ok: true, error: null };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : 'JSON 语法错误' };
  }
}

watch(
  () => props.modelValue,
  body => {
    if (body.kind === 'json') {
      jsonValidation.value = validateJson(body.text);
    } else {
      jsonValidation.value = { ok: true, error: null };
    }
  },
  { immediate: true, deep: true },
);

function setKind(kind: ApiClientBodyKind | string | number): void {
  const nextKind = kind as ApiClientBodyKind;
  if (nextKind === currentKind.value) {
    return;
  }
  emit('update:modelValue', {
    kind: nextKind,
    text: props.modelValue.text,
    form: props.modelValue.form,
  });
}

const kindProxy = computed<ApiClientBodyKind>({
  get: () => currentKind.value,
  set: value => setKind(value),
});

function updateText(text: string | number): void {
  emit('update:modelValue', {
    ...props.modelValue,
    text: String(text ?? ''),
  });
}

function handleTextInput(value: string | number): void {
  updateText(value);
}

function handleFormUpdate(rows: ApiClientRequestBody['form']): void {
  updateForm(rows);
}

function updateForm(rows: ApiClientRequestBody['form']): void {
  emit('update:modelValue', {
    ...props.modelValue,
    form: rows,
  });
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-3">
      <span class="text-sm font-medium text-foreground">Body 类型</span>
      <NativeSelect
        v-model="kindProxy"
        class="min-w-[140px]"
      >
        <NativeSelectOption value="none">
          无请求体
        </NativeSelectOption>
        <NativeSelectOption value="json">
          JSON
        </NativeSelectOption>
        <NativeSelectOption value="text">
          原始文本
        </NativeSelectOption>
        <NativeSelectOption value="form">
          URL 编码表单
        </NativeSelectOption>
      </NativeSelect>
    </div>

    <Tabs :model-value="currentKind" class="w-full">
      <TabsList class="hidden" />
      <TabsContent value="none" class="mt-0">
        <div class="rounded-md border border-dashed bg-muted/20 px-3 py-6 text-center text-sm text-muted-foreground">
          当前请求无 Body
        </div>
      </TabsContent>
      <TabsContent value="json" class="mt-0">
        <div class="flex flex-col gap-2">
          <Textarea
            :model-value="modelValue.text"
            placeholder="{&quot;key&quot;: &quot;value&quot;}"
            :rows="10"
            class="font-mono text-xs"
            @update:model-value="handleTextInput"
          />
          <Alert v-if="!jsonValidation.ok" variant="destructive">
            <AlertTriangle class="size-4" />
            <AlertTitle>JSON 语法错误</AlertTitle>
            <AlertDescription>
              {{ jsonValidation.error }}
            </AlertDescription>
          </Alert>
        </div>
      </TabsContent>
      <TabsContent value="text" class="mt-0">
        <Textarea
          :model-value="modelValue.text"
          placeholder="原始文本内容"
          :rows="10"
          class="font-mono text-xs"
          @update:model-value="handleTextInput"
        />
      </TabsContent>
      <TabsContent value="form" class="mt-0">
        <KeyValueEditor
          :model-value="modelValue.form"
          key-placeholder="字段"
          value-placeholder="值"
          add-label="新增字段"
          empty-hint="URL 编码表单为空"
          @update:model-value="handleFormUpdate"
        />
      </TabsContent>
    </Tabs>
  </div>
</template>

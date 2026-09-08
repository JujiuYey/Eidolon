<script setup lang="ts">
import { AlertTriangle, CheckCircle2, RotateCcw, Sparkles, Square } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import type { ApiClientAiCandidate, ApiClientRequestBody } from '@/types/api-client';

interface Props {
  candidate: ApiClientAiCandidate | null;
  isGenerating: boolean;
  currentBody: ApiClientRequestBody;
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  currentModelLabel: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:prompt', value: string): void;
  (e: 'update:reference', value: string): void;
  (e: 'update:includeCurrentBody', value: boolean): void;
  (e: 'generate'): void;
  (e: 'cancel'): void;
  (e: 'apply'): void;
  (e: 'edit', value: string): void;
}>();

const localCandidateContent = ref('');

watch(
  () => props.candidate?.content,
  next => {
    localCandidateContent.value = next ?? '';
  },
  { immediate: true },
);

const jsonState = computed(() => {
  const trimmed = localCandidateContent.value.trim();
  if (!trimmed) {
    return { ok: false, error: '候选为空' };
  }
  try {
    JSON.parse(trimmed);
    return { ok: true, error: null };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : 'JSON 语法错误' };
  }
});

const applyDisabled = computed(() => {
  if (!props.candidate || props.candidate.status !== 'success') {
    return true;
  }
  if (!jsonState.value.ok) {
    return true;
  }
  if (localCandidateContent.value !== (props.candidate?.content ?? '')) {
    return false;
  }
  return false;
});

function handleApply(): void {
  if (!props.candidate || props.candidate.status !== 'success') {
    return;
  }
  if (!jsonState.value.ok) {
    toast.error('候选 JSON 校验未通过，无法应用');
    return;
  }
  emit('apply');
  toast.success('已应用候选内容到 Body（未保存）');
}

function handleEdit(value: string | number): void {
  const next = String(value ?? '');
  localCandidateContent.value = next;
  emit('edit', next);
}

function handleCancel(): void {
  emit('cancel');
}

function handleGenerate(): void {
  if (!props.prompt.trim()) {
    toast.error('请先填写生成要求');
    return;
  }
  emit('generate');
}
</script>

<template>
  <div class="flex flex-col gap-3 rounded-lg border bg-card p-4">
    <div class="flex items-center gap-2">
      <Sparkles class="size-4 text-primary" />
      <h3 class="text-sm font-semibold">
        AI 请求体生成
      </h3>
      <Badge v-if="candidate?.status === 'generating'" variant="secondary">
        生成中
      </Badge>
      <Badge v-else-if="candidate?.status === 'success'" variant="default">
        就绪
      </Badge>
      <Badge v-else-if="candidate?.status === 'error'" variant="destructive">
        失败
      </Badge>
      <Badge v-else-if="candidate?.status === 'cancelled'" variant="outline">
        已取消
      </Badge>
    </div>

    <div class="flex flex-col gap-2">
      <Label>生成要求</Label>
      <Textarea
        :model-value="prompt"
        placeholder="例如：生成一个订单，包含三件商品，金额保留两位小数，收货地址使用中文。"
        :rows="3"
        @update:model-value="value => emit('update:prompt', String(value ?? ''))"
      />
    </div>

    <div class="flex flex-col gap-2">
      <Label>参考内容</Label>
      <Textarea
        :model-value="reference"
        placeholder="可粘贴字段说明、接口文档片段或 JSON 示例"
        :rows="4"
        @update:model-value="value => emit('update:reference', String(value ?? ''))"
      />
    </div>

    <div class="flex items-center gap-2">
      <Switch
        id="include-current-body"
        :model-value="includeCurrentBody"
        @update:model-value="value => emit('update:includeCurrentBody', Boolean(value))"
      />
      <Label for="include-current-body" class="cursor-pointer text-xs">
        带入当前 Body（仅在 Body 为 JSON 时生效）
      </Label>
    </div>

    <div class="flex items-center gap-2">
      <Label class="w-20 shrink-0">
        模型
      </Label>
      <div class="flex-1 rounded-md border bg-muted/30 px-3 py-1.5 text-sm">
        <span v-if="currentModelLabel">{{ currentModelLabel }}</span>
        <span v-else class="text-muted-foreground">
          等待生成结果以显示当前使用的模型
        </span>
      </div>
    </div>

    <div class="flex gap-2">
      <Button v-if="!isGenerating" :disabled="!prompt.trim()" @click="handleGenerate">
        <Sparkles class="size-4" />
        生成
      </Button>
      <Button v-else variant="destructive" @click="handleCancel">
        <Square class="size-4" />
        取消生成
      </Button>
      <Button variant="ghost" :disabled="!candidate" @click="emit('edit', candidate?.content ?? '')">
        <RotateCcw class="size-4" />
        还原候选
      </Button>
    </div>

    <Alert v-if="candidate?.status === 'error' || candidate?.status === 'cancelled'" variant="destructive">
      <AlertTriangle class="size-4" />
      <AlertTitle>{{ candidate.status === 'error' ? '生成失败' : '已取消' }}</AlertTitle>
      <AlertDescription>{{ candidate.errorMessage ?? (candidate.status === 'cancelled' ? '生成被取消，原内容保留' : '请稍后重试') }}</AlertDescription>
    </Alert>

    <div v-if="isGenerating || candidate" class="flex flex-col gap-2">
      <Label>候选内容（不会自动应用）</Label>
      <Textarea
        :model-value="localCandidateContent"
        placeholder="生成结果将显示在此处，可继续编辑"
        :rows="8"
        class="font-mono text-xs"
        :disabled="isGenerating"
        @update:model-value="handleEdit"
      />

      <div v-if="!isGenerating && localCandidateContent" class="flex items-center gap-2 text-xs">
        <CheckCircle2 v-if="jsonState.ok" class="size-4 text-emerald-500" />
        <AlertTriangle v-else class="size-4 text-destructive" />
        <span :class="jsonState.ok ? 'text-emerald-500' : 'text-destructive'">
          {{ jsonState.ok ? 'JSON 合法' : jsonState.error }}
        </span>
      </div>

      <Button :disabled="applyDisabled" @click="handleApply">
        <Sparkles class="size-4" />
        应用到请求体
      </Button>
    </div>

    <Alert>
      <AlertTitle>使用后端默认模型</AlertTitle>
      <AlertDescription>请求体生成使用后端「默认模型」页签中 `api_body_generation` 键对应的模型。前端无法列出或切换；如需修改，请在设置页调整默认模型。</AlertDescription>
    </Alert>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { AlertCircle, Layers, Loader2, Plus, Trash2 } from 'lucide-vue-next';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
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
import { useConfirm } from '@/composables/use-confirm';
import { useApiClientStore } from '@/stores/api-client';
import type { ApiClientEnvironment, ApiClientKeyValueRow } from '@/types/api-client';
import KeyValueEditor from './KeyValueEditor.vue';

interface Props {
  open: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
}>();

const store = useApiClientStore();
const confirm = useConfirm();

const selectedId = ref<string | null>(null);
const isCreating = ref(false);
const draftName = ref('');
const draftBaseUrl = ref('');
const draftVariables = ref<ApiClientKeyValueRow[]>([]);
const saving = ref(false);

const environments = computed(() => store.environments);
const isEditingExisting = computed(() =>
  !isCreating.value && selectedId.value !== null && environments.value.some(env => env.id === selectedId.value),
);

const showEditor = computed(() => isCreating.value || isEditingExisting.value);

const selectedEnvironment = computed(() =>
  selectedId.value
    ? environments.value.find(env => env.id === selectedId.value) ?? null
    : null,
);

const dirty = computed(() => {
  if (!isEditingExisting.value || !selectedEnvironment.value) {
    return draftName.value.trim().length > 0 || draftBaseUrl.value.trim().length > 0 || draftVariables.value.length > 0;
  }
  const env = selectedEnvironment.value;
  return (
    env.name !== draftName.value.trim()
    || env.baseUrl !== draftBaseUrl.value.trim()
    || JSON.stringify(env.variables) !== JSON.stringify(draftVariables.value)
  );
});

const canSave = computed(() => {
  if (saving.value || store.isMutatingEnvironment) {
    return false;
  }
  return draftName.value.trim().length > 0 && dirty.value;
});

function generateId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `env_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;
}

function resetDraft(): void {
  draftName.value = '';
  draftBaseUrl.value = '';
  draftVariables.value = [];
}

function loadFromEnvironment(env: ApiClientEnvironment): void {
  draftName.value = env.name;
  draftBaseUrl.value = env.baseUrl;
  draftVariables.value = env.variables.map(row => ({ ...row }));
}

function handleSelect(id: string): void {
  isCreating.value = false;
  selectedId.value = id;
  const env = environments.value.find(item => item.id === id);
  if (env) {
    loadFromEnvironment(env);
  } else {
    resetDraft();
  }
}

function handleCreateNew(): void {
  isCreating.value = true;
  selectedId.value = null;
  resetDraft();
}

function syncSelectionToEnvironments(): void {
  if (selectedId.value && !environments.value.some(env => env.id === selectedId.value)) {
    selectedId.value = environments.value[0]?.id ?? null;
    if (selectedId.value) {
      const next = environments.value.find(env => env.id === selectedId.value);
      if (next) {
        loadFromEnvironment(next);
      }
    } else {
      resetDraft();
    }
  }
}

async function persistDraft(): Promise<void> {
  if (!canSave.value) {
    return;
  }
  saving.value = true;
  try {
    const id = selectedId.value ?? generateId();
    const payload: ApiClientEnvironment = {
      id,
      projectId: store.activeProjectId ?? '',
      name: draftName.value.trim(),
      baseUrl: draftBaseUrl.value.trim(),
      variables: draftVariables.value,
      createdAt: selectedEnvironment.value?.createdAt ?? Date.now(),
      updatedAt: Date.now(),
    };
    await store.upsertEnvironment(payload);
    selectedId.value = id;
    isCreating.value = false;
    if (!store.activeEnvironmentId) {
      store.selectEnvironment(id);
    }
  } finally {
    saving.value = false;
  }
}

async function handleDelete(): Promise<void> {
  if (!selectedEnvironment.value) {
    return;
  }
  const env = selectedEnvironment.value;
  const ok = await confirm.ask({
    title: '删除环境',
    description: `确定删除环境 “${env.name}” 吗？该操作不可撤销。`,
    confirmLabel: '删除',
    cancelLabel: '取消',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  await store.removeEnvironment(env.id);
  syncSelectionToEnvironments();
}

function handleOpenChange(open: boolean): void {
  emit('update:open', open);
  if (!open) {
    selectedId.value = null;
    isCreating.value = false;
    resetDraft();
    saving.value = false;
  }
}

watch(
  () => [props.open, environments.value.length] as const,
  ([open, length]) => {
    if (!open) {
      return;
    }
    if (length === 0) {
      selectedId.value = null;
      resetDraft();
      return;
    }
    if (isCreating.value) {
      // Stay in create mode while the user fills the form. After save the
      // persistDraft handler clears isCreating and points at the new id.
      return;
    }
    if (selectedId.value && environments.value.some(env => env.id === selectedId.value)) {
      const env = environments.value.find(item => item.id === selectedId.value);
      if (env) {
        loadFromEnvironment(env);
      }
      return;
    }
    selectedId.value = environments.value[0]?.id ?? null;
    if (selectedId.value) {
      const env = environments.value.find(item => item.id === selectedId.value);
      if (env) {
        loadFromEnvironment(env);
      }
    }
  },
  { immediate: true },
);
</script>

<template>
  <Dialog :open="props.open" @update:open="handleOpenChange">
    <DialogContent class="flex h-[600px] max-h-[85vh] flex-col gap-0 sm:max-w-3xl">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <Layers class="size-4 text-primary" />
          环境管理
        </DialogTitle>
        <DialogDescription>
          为当前项目维护多个环境，每个环境包含 baseUrl 与变量集合。保存后立即生效。
        </DialogDescription>
      </DialogHeader>

      <div class="mt-3 grid min-h-0 flex-1 grid-cols-[200px_minmax(0,1fr)] gap-3 overflow-hidden">
        <div class="flex min-h-0 flex-col gap-2 border-r pr-3">
          <Button variant="outline" size="sm" class="w-full justify-start" @click="handleCreateNew">
            <Plus class="size-4" />
            新建环境
          </Button>
          <ScrollArea class="min-h-0 flex-1">
            <div v-if="environments.length === 0" class="py-4 text-center text-xs text-muted-foreground">
              暂无环境，点击上方按钮创建。
            </div>
            <ul v-else class="flex flex-col gap-1">
              <li v-for="env of environments" :key="env.id">
                <button
                  type="button"
                  class="w-full rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-muted/60"
                  :class="env.id === selectedId ? 'bg-muted font-medium text-foreground' : 'text-muted-foreground'"
                  @click="handleSelect(env.id)"
                >
                  <div class="truncate">
                    {{ env.name }}
                  </div>
                  <div class="truncate text-[11px] text-muted-foreground/80">
                    {{ env.baseUrl || '—' }}
                  </div>
                </button>
              </li>
            </ul>
          </ScrollArea>
        </div>

        <div class="flex min-h-0 flex-col gap-3 overflow-hidden">
          <div v-if="!showEditor" class="flex flex-1 items-center justify-center text-sm text-muted-foreground">
            选择左侧环境以编辑，或点击“新建环境”。
          </div>

          <ScrollArea v-else class="min-h-0 flex-1 pr-1">
            <div class="flex flex-col gap-4 p-2">
              <div
                v-if="isCreating"
                class="flex items-start gap-2 rounded-md border border-primary/40 bg-primary/10 px-3 py-2 text-xs font-medium text-primary"
                role="status"
              >
                <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
                <span>正在创建新环境，填写后点击底部“创建环境”以保存。</span>
              </div>
              <div class="grid gap-2">
                <Label for="env-name">名称</Label>
                <Input
                  id="env-name"
                  v-model="draftName"
                  placeholder="例如：开发 / 预发 / 生产"
                />
              </div>

              <div class="grid gap-2">
                <Label for="env-base-url">Base URL</Label>
                <Input
                  id="env-base-url"
                  v-model="draftBaseUrl"
                  placeholder="https://api.example.com"
                />
              </div>

              <div class="grid gap-2">
                <Label>变量</Label>
                <KeyValueEditor
                  v-model="draftVariables"
                  key-placeholder="变量名"
                  value-placeholder="变量值"
                  add-label="新增变量"
                  empty-hint="暂无变量，点击下方按钮添加"
                />
              </div>
            </div>
          </ScrollArea>
        </div>
      </div>

      <DialogFooter class="mt-3 flex items-center justify-between gap-2 sm:justify-between">
        <Button
          variant="destructive"
          size="sm"
          :disabled="!isEditingExisting || saving || store.isMutatingEnvironment"
          @click="handleDelete"
        >
          <Trash2 class="size-4" />
          删除当前环境
        </Button>

        <div class="flex items-center gap-2">
          <Button variant="outline" size="sm" @click="handleOpenChange(false)">
            关闭
          </Button>
          <Button
            size="sm"
            :disabled="!canSave"
            @click="persistDraft"
          >
            <Loader2 v-if="saving || store.isMutatingEnvironment" class="size-4 animate-spin" />
            {{ isEditingExisting ? '保存修改' : '创建环境' }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <ConfirmDialog
    :open="confirm.state.open"
    :title="confirm.state.title"
    :description="confirm.state.description"
    :confirm-label="confirm.state.confirmLabel"
    :cancel-label="confirm.state.cancelLabel"
    :destructive="confirm.state.destructive"
    @update:open="confirm.onOpenChange"
    @confirm="confirm.onConfirm"
    @cancel="confirm.onCancel"
  />
</template>

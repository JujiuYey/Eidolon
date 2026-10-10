<script setup lang="ts">
import { Check, Trash2 } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';

import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { deleteGitAccount, upsertGitAccount } from '@/services';
import type { GitAccount } from '@/types/git';
import { getErrorMessage } from '@/utils/helpers';

const props = defineProps<{
  /** null 表示新建账户 */
  account: GitAccount | null;
}>();

const emit = defineEmits<{
  (e: 'saved', accountId: string): void;
  (e: 'removed', accountId: string): void;
}>();

const deleteConfirmOpen = ref(false);
const saving = ref(false);

function buildFormData(): GitAccount {
  return props.account
    ? { ...props.account }
    : {
        id: '',
        name: '',
        email: '',
        enabled: true,
        sort: 0,
        created_at: 0,
        updated_at: 0,
      };
}

const form = ref<GitAccount>(buildFormData());

watch(
  () => props.account,
  () => {
    form.value = buildFormData();
  },
);

const isFormValid = computed(() => form.value.name.trim() !== '' && form.value.email.trim().includes('@'));

async function handleSave() {
  saving.value = true;
  try {
    const saved = await upsertGitAccount({
      ...form.value,
      name: form.value.name.trim(),
      email: form.value.email.trim(),
    });
    toast.success('保存成功');
    emit('saved', saved.id);
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  } finally {
    saving.value = false;
  }
}

async function confirmDelete() {
  try {
    await deleteGitAccount(form.value.id);
    deleteConfirmOpen.value = false;
    toast.success('删除成功');
    emit('removed', form.value.id);
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex items-start justify-between gap-4 border-b px-6 py-5">
      <div class="min-w-0 space-y-1">
        <h3 class="truncate text-lg font-semibold tracking-tight text-foreground">
          {{ props.account ? props.account.name : '新建 Git 账号' }}
        </h3>
        <p class="text-xs text-muted-foreground">记录你在各台电脑上的 git 提交身份，仅保存在本机。</p>
      </div>

      <label class="flex shrink-0 items-center gap-2 pt-1 text-sm text-muted-foreground">
        启用
        <Switch v-model="form.enabled" />
      </label>
    </div>

    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
      <div class="grid gap-4 md:grid-cols-2">
        <div class="space-y-2">
          <Label class="text-sm font-semibold">显示名</Label>
          <Input v-model="form.name" placeholder="例如: 公司电脑" />
        </div>
        <div class="space-y-2">
          <Label class="text-sm font-semibold">Git 邮箱</Label>
          <Input v-model="form.email" placeholder="例如: me@company.com" autocomplete="off" />
        </div>
      </div>

      <Separator />

      <p class="text-xs text-muted-foreground">
        周报「只看我的提交」按启用账号的邮箱匹配提交作者。多台电脑用不同邮箱提交时，把每个邮箱都加进来即可。
      </p>
    </div>

    <div class="flex flex-wrap items-center justify-end gap-2 border-t px-6 py-4">
      <Button v-if="props.account" type="button" variant="destructive" @click="deleteConfirmOpen = true">
        <Trash2 class="mr-2 h-4 w-4" />
        删除账户
      </Button>

      <Button type="button" :disabled="!isFormValid || saving" @click="handleSave">
        <Check class="mr-2 h-4 w-4" />
        保存账户
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除该 Git 账号吗？"
    description="删除后，对应邮箱的提交将不再计入周报「只看我的提交」。"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

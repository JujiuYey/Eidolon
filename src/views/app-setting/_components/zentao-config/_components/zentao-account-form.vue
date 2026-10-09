<script setup lang="ts">
import { Check, Loader2, Trash2 } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import {
  deleteZentaoAccount,
  testZentaoConnection,
  upsertZentaoAccount,
} from '@/services';
import type { ZentaoAccount } from '@/types/zentao';
import { getErrorMessage } from '@/utils/helpers';

const props = defineProps<{
  /** null 表示新建账户 */
  account: ZentaoAccount | null;
}>();

const emit = defineEmits<{
  (e: 'saved', accountId: string): void;
  (e: 'removed', accountId: string): void;
}>();

const deleteConfirmOpen = ref(false);
const testing = ref(false);
const saving = ref(false);

function buildFormData(): ZentaoAccount {
  return props.account
    ? { ...props.account }
    : {
        id: '',
        name: '',
        base_url: '',
        account: '',
        password: '',
        enabled: true,
        sort: 0,
        created_at: 0,
        updated_at: 0,
      };
}

const form = ref<ZentaoAccount>(buildFormData());

watch(() => props.account, () => {
  form.value = buildFormData();
});

const isFormValid = computed(
  () =>
    form.value.name.trim() !== ''
    && /^https?:\/\/.+/.test(form.value.base_url.trim())
    && form.value.account.trim() !== ''
    && form.value.password !== '',
);

const canTestConnection = computed(
  () =>
    /^https?:\/\/.+/.test(form.value.base_url.trim())
    && form.value.account.trim() !== ''
    && form.value.password !== '',
);

async function handleTestConnection() {
  testing.value = true;
  try {
    await testZentaoConnection({
      base_url: form.value.base_url.trim(),
      account: form.value.account.trim(),
      password: form.value.password,
    });
    toast.success('禅道登录成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '禅道登录失败，请检查站点地址与凭据'));
  } finally {
    testing.value = false;
  }
}

async function handleSave() {
  saving.value = true;
  try {
    const saved = await upsertZentaoAccount({
      ...form.value,
      name: form.value.name.trim(),
      base_url: form.value.base_url.trim(),
      account: form.value.account.trim(),
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
    await deleteZentaoAccount(form.value.id);
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
          {{ props.account ? props.account.name : '新建禅道账户' }}
        </h3>
        <p class="text-xs text-muted-foreground">
          配置禅道站点与登录凭据，凭据仅保存在本机。
        </p>
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
          <Input
            v-model="form.name"
            placeholder="例如: 公司禅道"
          />
        </div>
        <div class="space-y-2">
          <Label class="text-sm font-semibold">登录账号</Label>
          <Input
            v-model="form.account"
            placeholder="禅道账号"
            autocomplete="off"
          />
        </div>
      </div>

      <div class="space-y-2">
        <Label class="text-sm font-semibold">站点地址</Label>
        <Input
          v-model="form.base_url"
          placeholder="例如: http://192.168.10.209"
        />
        <p class="text-xs text-muted-foreground">
          浏览器里禅道地址中 host 部分，可带子路径，如 http://zentao.company.com
        </p>
      </div>

      <div class="space-y-2">
        <Label class="text-sm font-semibold">登录密码</Label>
        <Input
          v-model="form.password"
          type="password"
          placeholder="禅道密码"
        />
        <p class="text-xs text-muted-foreground">
          凭据仅保存在本机，遵循项目「掩码而非加密」约定。
        </p>
      </div>

      <Separator />

      <p class="text-xs text-muted-foreground">
        配置完成后，在「禅道」页面选择该账户即可查看名下的任务与 Bug。
      </p>
    </div>

    <div class="flex flex-wrap items-center justify-end gap-2 border-t px-6 py-4">
      <Button
        type="button"
        variant="outline"
        :disabled="!canTestConnection || testing"
        @click="handleTestConnection"
      >
        <Loader2
          v-if="testing"
          class="mr-2 h-4 w-4 animate-spin"
        />
        {{ testing ? '登录中…' : '测试连接' }}
      </Button>

      <Button
        v-if="props.account"
        type="button"
        variant="destructive"
        @click="deleteConfirmOpen = true"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除账户
      </Button>

      <Button
        type="button"
        :disabled="!isFormValid || saving"
        @click="handleSave"
      >
        <Check class="mr-2 h-4 w-4" />
        保存账户
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除该禅道账户吗？"
    description="删除后需要重新配置才能在禅道页拉取任务。"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

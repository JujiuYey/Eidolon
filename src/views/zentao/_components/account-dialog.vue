<script setup lang="ts">
import type { ZentaoAccount } from '@/types/zentao';
import { computed, ref, watch } from 'vue';
import { Trash2 } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { useConfirm } from '@/composables/use-confirm';
import { testZentaoConnection } from '@/services/zentao';
import { useZentaoStore } from '@/stores/zentao';
import { getErrorMessage } from '@/utils/helpers';

const props = defineProps<{
  /** 打开时编辑的账户；为空表示新建 */
  account?: ZentaoAccount | null;
}>();

const open = defineModel<boolean>('open', { required: true });

const store = useZentaoStore();
const confirm = useConfirm();

const form = ref({ id: '', name: '', base_url: '', account: '', password: '' });
const saving = ref(false);
const testing = ref(false);

const isEditing = computed(() => form.value.id !== '');
const canSave = computed(
  () =>
    form.value.name.trim() !== ''
    && form.value.base_url.trim() !== ''
    && form.value.account.trim() !== ''
    && form.value.password !== '',
);

watch(open, isOpen => {
  if (!isOpen) {
    return;
  }
  if (props.account) {
    startEdit(props.account);
  } else {
    startCreate();
  }
});

function startCreate() {
  form.value = { id: '', name: '', base_url: '', account: '', password: '' };
}

function startEdit(account: ZentaoAccount) {
  form.value = {
    id: account.id,
    name: account.name,
    base_url: account.base_url,
    account: account.account,
    password: account.password,
  };
}

function buildPayload() {
  return {
    id: form.value.id,
    name: form.value.name.trim(),
    base_url: form.value.base_url.trim(),
    account: form.value.account.trim(),
    password: form.value.password,
    enabled: props.account?.enabled ?? true,
    sort: props.account?.sort ?? 0,
    created_at: props.account?.created_at ?? 0,
    updated_at: props.account?.updated_at ?? 0,
  };
}

async function handleTest() {
  testing.value = true;
  try {
    const result = await testZentaoConnection({
      base_url: form.value.base_url.trim(),
      account: form.value.account.trim(),
      password: form.value.password,
    });
    toast.success(result.message || '禅道登录成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '禅道登录失败，请检查站点地址与凭据'));
  } finally {
    testing.value = false;
  }
}

async function handleSave() {
  if (!canSave.value || saving.value) {
    return;
  }
  saving.value = true;
  try {
    const saved = await store.saveAccount(buildPayload());
    toast.success('账户已保存');
    startEdit(saved);
    open.value = false;
  } catch (error) {
    toast.error(getErrorMessage(error, '保存禅道账户失败'));
  } finally {
    saving.value = false;
  }
}

async function handleDelete() {
  if (!isEditing.value) {
    return;
  }
  const ok = await confirm.ask({
    title: '删除禅道账户',
    description: `确定删除「${form.value.name}」？删除后需要重新配置才能拉取任务。`,
    confirmLabel: '删除',
    destructive: true,
  });
  if (!ok) {
    return;
  }
  try {
    await store.removeAccount(form.value.id);
    toast.success('账户已删除');
    if (store.accounts[0]) {
      startEdit(store.accounts[0]);
    } else {
      startCreate();
      open.value = false;
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '删除禅道账户失败'));
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>{{ isEditing ? '编辑禅道账户' : '新建禅道账户' }}</DialogTitle>
        <DialogDescription>
          填写禅道站点地址与登录凭据，保存后即可拉取名下任务与 Bug。
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        <div class="space-y-2">
          <Label class="text-sm font-medium">显示名</Label>
          <Input
            v-model="form.name"
            placeholder="例如: 公司禅道"
            class="h-10"
          />
        </div>

        <div class="space-y-2">
          <Label class="text-sm font-medium">站点地址</Label>
          <Input
            v-model="form.base_url"
            placeholder="例如: http://192.168.10.209"
            class="h-10"
          />
          <p class="text-xs text-muted-foreground">
            浏览器里"我的地盘"地址中 host 部分，可带子路径。
          </p>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="space-y-2">
            <Label class="text-sm font-medium">登录账号</Label>
            <Input
              v-model="form.account"
              placeholder="禅道账号"
              class="h-10"
            />
          </div>
          <div class="space-y-2">
            <Label class="text-sm font-medium">登录密码</Label>
            <Input
              v-model="form.password"
              type="password"
              placeholder="禅道密码"
              class="h-10"
            />
          </div>
        </div>
      </div>

      <Separator />

      <DialogFooter class="flex items-center gap-2 sm:justify-between">
        <Button
          v-if="isEditing"
          type="button"
          variant="ghost"
          size="sm"
          class="text-destructive hover:bg-destructive/10 hover:text-destructive"
          @click="handleDelete"
        >
          <Trash2 class="mr-1 h-4 w-4" />
          删除
        </Button>
        <span v-else />

        <div class="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            :disabled="!form.base_url.trim() || !form.account.trim() || testing"
            @click="handleTest"
          >
            {{ testing ? '登录中…' : '测试连接' }}
          </Button>
          <Button
            type="button"
            size="sm"
            :disabled="!canSave || saving"
            @click="handleSave"
          >
            {{ saving ? '保存中…' : '保存' }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

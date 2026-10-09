<script setup lang="ts">
import { Check, Eye, EyeOff, SendHorizonal, Trash2 } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { SMTP_REGISTRY } from '@/config/smtp-registry';
import {
  deleteSmtpAccount,
  testSmtpConnection,
  upsertSmtpAccount,
} from '@/services';
import type { SmtpAccount, SmtpEncryption } from '@/types/mail';
import { getErrorMessage } from '@/utils/helpers';

/**
 * SMTP 账户表单核心（字段 + 校验 + 测试连接 + 保存/删除）。
 *
 * 设置页与邮件页的账户对话框共用，本组件不含页面级标题，
 * 外层用页面框架或 Dialog 包裹。
 */

const props = defineProps<{
  /** null 表示新建账户 */
  account: SmtpAccount | null;
}>();

const emit = defineEmits<{
  (e: 'saved', accountId: string): void;
  (e: 'removed', accountId: string): void;
}>();

const showPassword = ref(false);
const deleteConfirmOpen = ref(false);
const testing = ref(false);
const saving = ref(false);

function buildFormData(): SmtpAccount {
  if (props.account) {
    return { ...props.account };
  }
  // 新建时默认选中阿里云企业邮箱预设（公司邮箱场景）
  const preset = SMTP_REGISTRY[0]!;
  return {
    id: '',
    name: '',
    email: '',
    password: '',
    host: preset.host,
    port: preset.port,
    encryption: preset.encryption,
    from_name: '',
    enabled: true,
    sort: 0,
    created_at: 0,
    updated_at: 0,
  };
}

const form = ref<SmtpAccount>(buildFormData());

watch(() => props.account, () => {
  form.value = buildFormData();
  showPassword.value = false;
});

/** 根据当前 host 匹配预设；匹配不上视为自定义 */
const selectedPresetId = computed(() => {
  const matched = SMTP_REGISTRY.find(preset => preset.host !== '' && preset.host === form.value.host);
  return matched?.preset_id ?? 'custom';
});

const selectedPreset = computed(() =>
  SMTP_REGISTRY.find(preset => preset.preset_id === selectedPresetId.value) ?? null,
);

function handlePresetChange(presetId: string) {
  const preset = SMTP_REGISTRY.find(item => item.preset_id === presetId);
  if (!preset) {
    return;
  }
  if (preset.preset_id !== 'custom') {
    form.value.host = preset.host;
    form.value.port = preset.port;
    form.value.encryption = preset.encryption;
  } else {
    form.value.host = '';
  }
}

const encryptionOptions: Array<{ value: SmtpEncryption; label: string }> = [
  { value: 'tls', label: 'SSL/TLS（465 端口常用）' },
  { value: 'starttls', label: 'STARTTLS（587 端口常用）' },
  { value: 'none', label: '不加密（仅限可信内网）' },
];

const isFormValid = computed(() =>
  Boolean(form.value.name.trim())
  && form.value.email.includes('@')
  && Boolean(form.value.host.trim())
  && form.value.port >= 1
  && form.value.port <= 65535,
);

const canTestConnection = computed(() =>
  form.value.email.includes('@')
  && Boolean(form.value.host.trim())
  && form.value.port >= 1
  && form.value.port <= 65535,
);

async function handleTestConnection() {
  testing.value = true;
  try {
    const response = await testSmtpConnection({
      email: form.value.email,
      password: form.value.password,
      host: form.value.host,
      port: form.value.port,
      encryption: form.value.encryption,
      from_name: form.value.from_name,
    });
    toast.success(response.message);
  } catch (error) {
    toast.error(getErrorMessage(error, '测试邮件发送失败'));
  } finally {
    testing.value = false;
  }
}

async function handleSave() {
  saving.value = true;
  try {
    const saved = await upsertSmtpAccount({
      ...form.value,
      name: form.value.name.trim(),
      email: form.value.email.trim(),
      host: form.value.host.trim(),
      from_name: form.value.from_name.trim(),
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
    await deleteSmtpAccount(form.value.id);
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
    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
      <section class="space-y-3">
        <div class="flex items-center justify-between">
          <Label class="text-sm font-semibold">服务商预设</Label>
          <label class="flex items-center gap-2 text-sm text-muted-foreground">
            启用
            <Switch v-model="form.enabled" />
          </label>
        </div>
        <Select
          :model-value="selectedPresetId"
          @update:model-value="value => handlePresetChange(String(value ?? ''))"
        >
          <SelectTrigger>
            <SelectValue placeholder="选择服务商" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="preset of SMTP_REGISTRY"
              :key="preset.preset_id"
              :value="preset.preset_id"
            >
              {{ preset.name }}
            </SelectItem>
          </SelectContent>
        </Select>
        <p
          v-if="selectedPreset?.hint"
          class="text-xs text-muted-foreground"
        >
          {{ selectedPreset.hint }}
        </p>
      </section>

      <div class="grid gap-4 md:grid-cols-2">
        <section class="space-y-3">
          <Label class="text-sm font-semibold">账户名称</Label>
          <Input
            v-model="form.name"
            type="text"
            placeholder="例如: 公司邮箱"
          />
        </section>

        <section class="space-y-3">
          <Label class="text-sm font-semibold">发件人显示名</Label>
          <Input
            v-model="form.from_name"
            type="text"
            placeholder="例如: 张三（可选）"
          />
        </section>
      </div>

      <section class="space-y-3">
        <Label class="text-sm font-semibold">邮箱地址</Label>
        <Input
          v-model="form.email"
          type="email"
          placeholder="例如: zhangsan@company.com"
        />
        <p class="text-xs text-muted-foreground">
          同时作为 SMTP 登录用户名与默认发件地址。
        </p>
      </section>

      <section class="space-y-3">
        <Label class="text-sm font-semibold">密码 / 授权码</Label>
        <div class="relative">
          <Input
            v-model="form.password"
            :type="showPassword ? 'text' : 'password'"
            placeholder="请输入 SMTP 密码或授权码"
            class="pr-11"
          />
          <button
            type="button"
            class="absolute top-1/2 right-3 flex -translate-y-1/2 items-center text-muted-foreground transition hover:text-foreground"
            @click="showPassword = !showPassword"
          >
            <Eye
              v-if="!showPassword"
              class="h-4 w-4"
            />
            <EyeOff
              v-else
              class="h-4 w-4"
            />
          </button>
        </div>
      </section>

      <Separator />

      <section class="grid gap-4 md:grid-cols-3">
        <div class="space-y-3 md:col-span-2">
          <Label class="text-sm font-semibold">SMTP 服务器</Label>
          <Input
            v-model="form.host"
            type="text"
            placeholder="例如: smtp.qiye.aliyun.com"
          />
        </div>
        <div class="space-y-3">
          <Label class="text-sm font-semibold">端口</Label>
          <Input
            v-model.number="form.port"
            type="number"
            :min="1"
            :max="65535"
          />
        </div>
      </section>

      <section class="space-y-3">
        <Label class="text-sm font-semibold">加密方式</Label>
        <Select v-model="form.encryption">
          <SelectTrigger>
            <SelectValue placeholder="选择加密方式" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option of encryptionOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>
      </section>
    </div>

    <div class="flex flex-wrap items-center justify-end gap-2 border-t px-6 py-3">
      <Button
        type="button"
        variant="outline"
        size="sm"
        :disabled="!canTestConnection || testing"
        @click="handleTestConnection"
      >
        <SendHorizonal class="mr-1 h-4 w-4" />
        {{ testing ? '发送中…' : '发送测试邮件' }}
      </Button>

      <Button
        v-if="props.account"
        type="button"
        variant="ghost"
        size="sm"
        class="text-destructive hover:bg-destructive/10 hover:text-destructive"
        @click="deleteConfirmOpen = true"
      >
        <Trash2 class="mr-1 h-4 w-4" />
        删除
      </Button>

      <Button
        type="button"
        size="sm"
        :disabled="!isFormValid || saving"
        @click="handleSave"
      >
        <Check class="mr-1 h-4 w-4" />
        {{ saving ? '保存中…' : '保存账户' }}
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除该邮件账户吗？"
    description="删除后发送历史会保留，但无法再用该账户发信。"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

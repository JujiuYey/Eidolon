<script setup lang="ts">
import type { SmtpAccount } from '@/types/mail';
import SagSmtpAccountForm from '@/components/sag/sag-smtp-account-form/index.vue';

/**
 * 设置页的邮件账户表单页框架（标题 + 表单核心）。
 * 字段与保存/测试/删除逻辑统一在 `sag-smtp-account-form`，
 * 邮件页的账户对话框复用同一组件。
 */

const props = defineProps<{
  /** null 表示新建账户 */
  account: SmtpAccount | null;
}>();

const emit = defineEmits<{
  (e: 'saved', accountId: string): void;
  (e: 'removed', accountId: string): void;
}>();
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-1">
        <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
          {{ props.account ? props.account.name : '新建邮件账户' }}
        </h2>
        <p class="text-sm text-muted-foreground">
          配置 SMTP 发信服务器与凭据，凭据仅保存在本机。
        </p>
      </div>
    </div>

    <SagSmtpAccountForm
      :account="props.account"
      @saved="id => emit('saved', id)"
      @removed="id => emit('removed', id)"
    />
  </div>
</template>

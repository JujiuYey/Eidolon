<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { Mail, Plus } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { listSmtpAccounts } from '@/services';
import type { SmtpAccount } from '@/types/mail';
import { getErrorMessage } from '@/utils/helpers';
import SmtpAccountList from './_components/smtp-account-list.vue';
import SmtpAccountForm from './_components/smtp-account-form.vue';

const accounts = ref<SmtpAccount[]>([]);
/** 空字符串表示新建账户 */
const selectedAccountId = ref('');

const selectedAccount = ref<SmtpAccount | null>(null);

async function loadData(keepSelection = true) {
  try {
    accounts.value = await listSmtpAccounts();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载邮件账户失败'));
    return;
  }

  if (!keepSelection || !accounts.value.some(account => account.id === selectedAccountId.value)) {
    selectedAccountId.value = accounts.value[0]?.id ?? '';
  }
  selectedAccount.value = accounts.value.find(account => account.id === selectedAccountId.value)
    ?? null;
}

function handleSelect(accountId: string) {
  selectedAccountId.value = accountId;
  selectedAccount.value = accounts.value.find(account => account.id === accountId) ?? null;
}

function handleCreate() {
  selectedAccountId.value = '';
  selectedAccount.value = null;
}

async function handleSaved(accountId: string) {
  await loadData();
  selectedAccountId.value = accountId;
  selectedAccount.value = accounts.value.find(account => account.id === accountId) ?? null;
}

async function handleRemoved(accountId: string) {
  await loadData(false);
  void accountId;
}

onMounted(() => {
  void loadData(false);
});
</script>

<template>
  <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card">
    <div class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10">
      <div class="flex items-center justify-between gap-2 px-4 py-3">
        <h2 class="flex items-center gap-2 text-sm font-semibold">
          <Mail class="h-4 w-4 text-primary" />
          邮件账户
        </h2>
        <Button
          size="sm"
          variant="outline"
          @click="handleCreate"
        >
          <Plus class="h-4 w-4" />
          新建
        </Button>
      </div>
      <SmtpAccountList
        v-model:selected-account-id="selectedAccountId"
        :accounts="accounts"
        @select="handleSelect"
      />
    </div>
    <SmtpAccountForm
      :key="selectedAccountId"
      :account="selectedAccount"
      @saved="handleSaved"
      @removed="handleRemoved"
    />
  </div>
</template>

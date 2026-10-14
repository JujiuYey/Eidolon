<script setup lang="ts">
import { GitBranch, Plus } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import { Button } from '@/components/ui/button';
import { listGitAccounts } from '@/services';
import type { GitAccount } from '@/types/git';
import { getErrorMessage } from '@/utils/helpers';

import GitAccountForm from './_components/git-account-form.vue';
import GitAccountList from './_components/git-account-list.vue';

const accounts = ref<GitAccount[]>([]);
/** 空字符串表示新建账户 */
const selectedAccountId = ref('');

const selectedAccount = ref<GitAccount | null>(null);

async function loadData(keepSelection = true) {
  try {
    accounts.value = await listGitAccounts();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载 Git 账号失败'));
    return;
  }

  if (!keepSelection || !accounts.value.some(account => account.id === selectedAccountId.value)) {
    selectedAccountId.value = accounts.value[0]?.id ?? '';
  }
  selectedAccount.value = accounts.value.find(account => account.id === selectedAccountId.value) ?? null;
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

async function handleRemoved() {
  await loadData(false);
}

onMounted(() => {
  void loadData(false);
});
</script>

<template>
  <div class="flex min-h-0 flex-1 overflow-hidden rounded-lg border bg-card">
    <div class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10">
      <div class="flex items-center justify-between gap-2 px-4 py-3">
        <h2 class="flex items-center gap-2 text-sm font-semibold">
          <GitBranch class="h-4 w-4 text-primary" />
          Git 账号
        </h2>
        <Button size="sm" variant="outline" @click="handleCreate">
          <Plus class="h-4 w-4" />
          新建
        </Button>
      </div>
      <GitAccountList v-model:selected-account-id="selectedAccountId" :accounts="accounts" @select="handleSelect" />
    </div>
    <GitAccountForm :key="selectedAccountId" :account="selectedAccount" @saved="handleSaved" @removed="handleRemoved" />
  </div>
</template>

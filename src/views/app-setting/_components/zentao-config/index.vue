<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { Plus } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { listZentaoAccounts } from '@/services';
import type { ZentaoAccount } from '@/types/zentao';
import { getErrorMessage } from '@/utils/helpers';
import ZentaoAccountForm from './_components/zentao-account-form.vue';
import ZentaoAccountList from './_components/zentao-account-list.vue';

const accounts = ref<ZentaoAccount[]>([]);
/** 空字符串表示新建账户 */
const selectedAccountId = ref('');

const selectedAccount = ref<ZentaoAccount | null>(null);

async function loadData(keepSelection = true) {
  try {
    accounts.value = await listZentaoAccounts();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载禅道账户失败'));
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

async function handleRemoved() {
  await loadData(false);
}

onMounted(() => {
  void loadData(false);
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <div class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10">
        <div class="p-3">
          <Button
            type="button"
            variant="outline"
            class="w-full justify-start gap-2"
            :class="selectedAccountId === '' && 'border-primary bg-primary/20 shadow-xs'"
            @click="handleCreate"
          >
            <Plus class="h-4 w-4" />
            新建账户
          </Button>
        </div>
        <ZentaoAccountList
          v-model:selected-account-id="selectedAccountId"
          :accounts="accounts"
          @select="handleSelect"
        />
      </div>
      <ZentaoAccountForm
        :key="selectedAccountId"
        :account="selectedAccount"
        @saved="handleSaved"
        @removed="handleRemoved"
      />
    </div>
  </div>
</template>

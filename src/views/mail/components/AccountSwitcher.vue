<script setup lang="ts">
import { computed } from 'vue';
import { cn } from '@/lib/utils';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { SmtpAccount } from '@/types/mail';

interface AccountSwitcherProps {
  isCollapsed: boolean;
  accounts: SmtpAccount[];
}

const props = defineProps<AccountSwitcherProps>();

const selectedAccountId = defineModel<string>('selectedAccountId', {
  required: true,
});

const selectedAccount = computed(() =>
  props.accounts.find(account => account.id === selectedAccountId.value) ?? null,
);
</script>

<template>
  <div
    :class="cn(
      'flex w-full flex-col',
      isCollapsed && 'items-center',
    )"
  >
    <Select v-model="selectedAccountId">
      <SelectTrigger
        aria-label="选择发件账户"
        :class="cn(
          'w-full items-center gap-2 [&>span]:line-clamp-1 [&>span]:flex [&>span]:w-full [&>span]:items-center [&>span]:gap-1 [&>span]:truncate',
          { 'h-9 w-9 shrink-0 justify-center p-0 [&>span]:w-auto [&>svg]:hidden': isCollapsed },
        )"
      >
        <SelectValue placeholder="选择发件账户">
          <div
            v-if="selectedAccount"
            class="flex min-w-0 items-center gap-2"
          >
            <span class="flex size-5 shrink-0 items-center justify-center rounded bg-primary/15 text-[10px] font-semibold uppercase text-primary">
              {{ selectedAccount.name.slice(0, 2) }}
            </span>
            <span
              v-if="!isCollapsed"
              class="truncate text-sm"
            >
              {{ selectedAccount.name }}
            </span>
          </div>
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectItem
          v-for="account of accounts"
          :key="account.id"
          :value="account.id"
        >
          <div class="flex items-center gap-2">
            <span class="truncate">
              {{ account.name }}
            </span>
            <span class="truncate text-xs text-muted-foreground">
              {{ account.email }}
            </span>
          </div>
        </SelectItem>
        <SelectItem
          v-if="accounts.length === 0"
          value="__empty__"
          disabled
        >
          暂无账户
        </SelectItem>
      </SelectContent>
    </Select>

    <p
      v-if="!isCollapsed && accounts.length === 0"
      class="px-1 text-xs leading-5 text-muted-foreground"
    >
      还没有发件账户，可点击下方「管理账户」配置。
    </p>
  </div>
</template>

<script setup lang="ts">
import type { ZentaoAccount } from '@/types/zentao';

defineProps<{
  accounts: ZentaoAccount[];
}>();

const emit = defineEmits<{
  (e: 'select', accountId: string): void;
}>();

const selectedAccountId = defineModel<string>('selectedAccountId', {
  required: true,
});

function handleSelect(accountId: string) {
  selectedAccountId.value = accountId;
  emit('select', accountId);
}
</script>

<template>
  <nav class="min-h-0 flex-1 space-y-1.5 overflow-y-auto px-3 pb-3">
    <button
      v-for="account of accounts"
      :key="account.id"
      type="button"
      class="flex w-full items-center gap-3 rounded-lg border px-3 py-3 text-left transition-colors"
      :class="selectedAccountId === account.id
        ? 'border-primary bg-primary/20 shadow-xs'
        : 'border-transparent hover:bg-primary/10'"
      @click="handleSelect(account.id)"
    >
      <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-background text-xs font-semibold uppercase text-muted-foreground ring-1 ring-black/5">
        {{ account.name.slice(0, 2) }}
      </div>

      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium text-foreground">
          {{ account.name }}
        </p>
        <p class="truncate text-xs text-muted-foreground">
          {{ account.base_url.replace(/^https?:\/\//, '') }}
        </p>
      </div>

      <span
        v-if="!account.enabled"
        class="shrink-0 rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
      >
        已停用
      </span>
    </button>

    <div
      v-if="accounts.length === 0"
      class="rounded-lg border border-dashed bg-muted/10 p-4 text-xs text-muted-foreground"
    >
      还没有禅道账户，点击上方「新建账户」开始配置。
    </div>
  </nav>
</template>

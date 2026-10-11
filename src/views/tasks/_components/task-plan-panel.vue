<script setup lang="ts">
import { Loader2, Sparkles, X } from 'lucide-vue-next';

import { Button } from '@/components/ui/button';
import type { AiPlanState } from '@/stores/tasks';

defineProps<{
  plan: AiPlanState;
}>();

const emit = defineEmits<{
  (e: 'dismiss'): void;
  (e: 'retry'): void;
}>();
</script>

<template>
  <section class="rounded-xl border border-primary/30 bg-primary/5 p-4" aria-live="polite">
    <div class="flex items-center gap-2">
      <Sparkles class="size-4 text-primary" />
      <h2 class="text-sm font-semibold">今日建议</h2>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        class="ml-auto size-7 text-muted-foreground"
        aria-label="关闭建议"
        @click="emit('dismiss')"
      >
        <X class="size-4" />
      </Button>
    </div>

    <div v-if="plan.status === 'loading'" class="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
      <Loader2 class="size-4 animate-spin" />
      正在思考…
    </div>

    <div v-else-if="plan.status === 'error'" class="mt-3 space-y-2">
      <p class="text-sm text-destructive">{{ plan.errorMessage }}</p>
      <Button type="button" variant="outline" size="sm" @click="emit('retry')"> 重试 </Button>
    </div>

    <p v-else class="mt-3 whitespace-pre-wrap text-sm leading-6">{{ plan.text }}</p>
  </section>
</template>

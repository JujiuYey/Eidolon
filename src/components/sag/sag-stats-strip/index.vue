<script setup lang="ts">
/** 统计瓦片：数字 + 标签（tone=danger 用于逾期/失败这类需要醒目的数字） */
export interface StatTile {
  label: string;
  value: string | number;
  /** 数字后的补充说明（单位或最近时间） */
  hint?: string;
  tone?: 'default' | 'danger';
}

defineProps<{
  tiles: StatTile[];
}>();
</script>

<template>
  <div class="rounded-xl border bg-card px-4 py-3">
    <div class="flex flex-wrap items-center gap-x-8 gap-y-3">
      <div v-for="tile of tiles" :key="tile.label" class="min-w-0">
        <p class="text-xs text-muted-foreground">{{ tile.label }}</p>
        <p class="mt-1 flex items-baseline gap-1.5">
          <span
            class="text-xl leading-none font-semibold tabular-nums"
            :class="tile.tone === 'danger' ? 'text-destructive' : ''"
            >{{ tile.value }}</span
          >
          <span v-if="tile.hint" class="text-xs text-muted-foreground">{{ tile.hint }}</span>
        </p>
      </div>
    </div>
  </div>
</template>

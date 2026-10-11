<script setup lang="ts">
import { computed } from 'vue';

/** 统计瓦片：数字 + 标签（tone=danger 用于逾期/失败这类需要醒目的数字） */
export interface StatTile {
  label: string;
  value: string | number;
  /** 数字后的补充说明（单位或最近时间） */
  hint?: string;
  tone?: 'default' | 'danger';
}

/** 迷你趋势条：values 与 tooltips 按天对齐，末位是今天 */
export interface StatTrend {
  label: string;
  values: number[];
  tooltips?: string[];
}

const props = defineProps<{
  tiles: StatTile[];
  trend?: StatTrend | null;
}>();

const trendTotal = computed(() => props.trend?.values.reduce((sum, value) => sum + value, 0) ?? 0);

const trendMax = computed(() => Math.max(1, ...(props.trend?.values ?? [1])));

function barHeight(value: number): string {
  if (value <= 0) {
    return '2px';
  }
  return `${Math.max(10, Math.round((value / trendMax.value) * 100))}%`;
}

const trendAriaLabel = computed(() => {
  if (!props.trend) {
    return '';
  }
  if (trendTotal.value === 0) {
    return `${props.trend.label}暂无记录`;
  }
  return `${props.trend.label}共 ${trendTotal.value} 条，最多的一天 ${trendMax.value} 条`;
});
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

      <template v-if="trend">
        <div class="hidden h-10 w-px shrink-0 bg-border lg:block" aria-hidden="true" />
        <div class="ml-auto w-full max-w-56 min-w-44 flex-1 lg:flex-none">
          <div class="flex items-center justify-between gap-2">
            <p class="text-xs text-muted-foreground">{{ trend.label }}</p>
            <p class="text-xs text-muted-foreground tabular-nums">{{ trendTotal }}</p>
          </div>
          <div class="mt-1.5 flex h-8 items-end gap-px" role="img" :aria-label="trendAriaLabel">
            <div
              v-for="(value, index) of trend.values"
              :key="index"
              class="min-w-0 flex-1 rounded-sm transition-colors"
              :class="value > 0 ? 'bg-primary/60 hover:bg-primary' : 'bg-muted'"
              :style="{ height: barHeight(value) }"
              :title="trend.tooltips?.[index]"
            />
          </div>
        </div>
      </template>
    </div>

    <div v-if="$slots.extra" class="mt-3 border-t pt-3">
      <slot name="extra" />
    </div>
  </div>
</template>

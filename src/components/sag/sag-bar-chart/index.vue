<script setup lang="ts">
import { computed } from 'vue';

/**
 * 每日柱状图（纯展示）：基线 + 中线 + 悬浮数值。
 * 值全为 0 时显示「暂无数据」，不画空坐标。
 */

const props = withDefaults(
  defineProps<{
    values: number[];
    /** 与 values 对齐的 x 轴标签（如「9月15日」），展示首/中/末三个 */
    labels?: string[];
    /** 与 values 对齐的悬浮文案 */
    tooltips?: string[];
    ariaLabel?: string;
  }>(),
  { labels: () => [], tooltips: () => [], ariaLabel: '' },
);

const total = computed(() => props.values.reduce((sum, value) => sum + value, 0));
const maxValue = computed(() => Math.max(0, ...props.values));

function barHeight(value: number): string {
  if (maxValue.value <= 0) {
    return '0%';
  }
  return `${Math.max(3, Math.round((value / maxValue.value) * 100))}%`;
}

/** 首/中/末三个刻度位置，值不足 3 个时按序取 */
const tickIndices = computed<number[]>(() => {
  const count = props.values.length;
  if (count === 0) {
    return [];
  }
  if (count <= 3) {
    return props.values.map((_, index) => index);
  }
  return [0, Math.floor((count - 1) / 2), count - 1];
});

const tickLabels = computed<string[]>(() => tickIndices.value.map(index => props.labels[index] ?? ''));

const ariaText = computed(() => {
  if (!props.ariaLabel) {
    return '';
  }
  return total.value === 0
    ? `${props.ariaLabel}：暂无数据`
    : `${props.ariaLabel}：共 ${total.value}，峰值 ${maxValue.value}`;
});
</script>

<template>
  <div v-if="total > 0" class="flex flex-col">
    <div class="flex items-end justify-between gap-2">
      <span class="text-xs text-muted-foreground">峰值 {{ maxValue }}</span>
      <span class="text-xs text-muted-foreground tabular-nums">合计 {{ total }}</span>
    </div>

    <div
      class="relative mt-1.5 flex h-36 items-end gap-0.5 border-b border-border/70"
      role="img"
      :aria-label="ariaText"
    >
      <div
        class="pointer-events-none absolute inset-x-0 top-0 border-t border-dashed border-border/50"
        aria-hidden="true"
      />
      <div
        class="pointer-events-none absolute inset-x-0 top-1/2 border-t border-dashed border-border/50"
        aria-hidden="true"
      />
      <div
        v-for="(value, index) of values"
        :key="index"
        class="min-w-0 flex-1 rounded-t-[3px] bg-primary/65 transition-[height,background-color] hover:bg-primary"
        :style="{ height: barHeight(value) }"
        :title="tooltips[index]"
      />
    </div>

    <div class="mt-1.5 flex justify-between text-[11px] text-muted-foreground tabular-nums">
      <span v-for="(label, index) of tickLabels" :key="index" class="min-w-0 truncate">{{ label }}</span>
    </div>
  </div>

  <div v-else class="flex h-36 items-center justify-center text-sm text-muted-foreground">暂无数据</div>
</template>

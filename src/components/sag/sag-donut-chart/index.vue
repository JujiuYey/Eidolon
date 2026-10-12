<script setup lang="ts">
import { computed } from 'vue';

import { donutSegments } from '@/utils/chart';

/** 图表语义色调：映射为 literal class（Tailwind 不支持拼接类名） */
export type DonutTone = 'emerald' | 'amber' | 'sky' | 'teal' | 'red' | 'violet' | 'orange' | 'slate';

const TONE_CLASSES: Record<DonutTone, { stroke: string; dot: string }> = {
  emerald: { stroke: 'stroke-emerald-500', dot: 'bg-emerald-500' },
  amber: { stroke: 'stroke-amber-500', dot: 'bg-amber-500' },
  sky: { stroke: 'stroke-sky-500', dot: 'bg-sky-500' },
  teal: { stroke: 'stroke-teal-500', dot: 'bg-teal-500' },
  red: { stroke: 'stroke-red-500', dot: 'bg-red-500' },
  violet: { stroke: 'stroke-violet-500', dot: 'bg-violet-500' },
  orange: { stroke: 'stroke-orange-500', dot: 'bg-orange-500' },
  slate: { stroke: 'stroke-slate-400', dot: 'bg-slate-400' },
};

export interface DonutSegment {
  label: string;
  value: number;
  tone: DonutTone;
}

const props = withDefaults(
  defineProps<{
    segments: DonutSegment[];
    /** 环中心的数值文案 */
    centerValue?: string;
    /** 环中心的说明文案 */
    centerLabel?: string;
    ariaLabel?: string;
  }>(),
  { centerValue: '', centerLabel: '', ariaLabel: '' },
);

const math = computed(() => donutSegments(props.segments));

interface RenderedSegment {
  label: string;
  value: number;
  percentLabel: string;
  dashArray: string;
  dashOffset: string;
  strokeClass: string;
  dotClass: string;
}

const rendered = computed<RenderedSegment[]>(() =>
  math.value.segments.flatMap(segment => {
    const tone = props.segments.find(item => item.label === segment.label)?.tone ?? 'slate';
    const classes = TONE_CLASSES[tone];
    return [
      {
        label: segment.label,
        value: segment.value,
        percentLabel: segment.percentLabel,
        dashArray: segment.dashArray,
        dashOffset: segment.dashOffset,
        strokeClass: classes.stroke,
        dotClass: classes.dot,
      },
    ];
  }),
);

const ariaText = computed(() => {
  if (!props.ariaLabel) {
    return '';
  }
  if (rendered.value.length === 0) {
    return `${props.ariaLabel}：暂无数据`;
  }
  const summary = rendered.value.map(item => `${item.label} ${item.value}（${item.percentLabel}）`).join('，');
  return `${props.ariaLabel}：${summary}`;
});
</script>

<template>
  <div v-if="rendered.length > 0" class="flex items-center gap-4">
    <div class="relative shrink-0">
      <svg viewBox="0 0 42 42" class="size-28" role="img" :aria-label="ariaText">
        <circle cx="21" cy="21" r="15.9155" fill="none" stroke="var(--color-muted)" stroke-width="6" />
        <circle
          v-for="segment of rendered"
          :key="segment.label"
          cx="21"
          cy="21"
          r="15.9155"
          fill="none"
          stroke-width="6"
          class="transition-[stroke-dasharray]"
          :class="segment.strokeClass"
          :stroke-dasharray="segment.dashArray"
          :stroke-dashoffset="segment.dashOffset"
        >
          <title>{{ `${segment.label} ${segment.value}（${segment.percentLabel}）` }}</title>
        </circle>
      </svg>
      <div class="absolute inset-0 flex flex-col items-center justify-center">
        <span class="text-lg leading-none font-semibold tabular-nums">{{ centerValue || math.total }}</span>
        <span v-if="centerLabel" class="mt-0.5 text-[11px] text-muted-foreground">{{ centerLabel }}</span>
      </div>
    </div>

    <ul class="flex min-w-0 flex-1 flex-col gap-1.5">
      <li v-for="segment of rendered" :key="segment.label" class="flex items-center gap-2 text-xs">
        <span class="size-2 shrink-0 rounded-full" :class="segment.dotClass" aria-hidden="true" />
        <span class="min-w-0 flex-1 truncate text-muted-foreground">{{ segment.label }}</span>
        <span class="tabular-nums">{{ segment.value }}</span>
        <span class="w-9 text-right text-muted-foreground tabular-nums">{{ segment.percentLabel }}</span>
      </li>
    </ul>
  </div>

  <div v-else class="flex h-28 items-center justify-center text-sm text-muted-foreground">暂无数据</div>
</template>

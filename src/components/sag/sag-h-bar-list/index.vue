<script setup lang="ts">
import { computed } from 'vue';

/**
 * 横向条形排行榜：标签 + 条 + 数值。条长按相对最大值归一。
 * items 可点选（如点击域名按其搜索）。
 */

export interface HBarItem {
  label: string;
  value: number;
}

const props = withDefaults(
  defineProps<{
    items: HBarItem[];
    /** 超出只显示前 N 条，默认全部 */
    limit?: number;
    selectable?: boolean;
  }>(),
  { limit: 0, selectable: false },
);

const emit = defineEmits<{
  (e: 'select', item: HBarItem): void;
}>();

const visible = computed(() => (props.limit > 0 ? props.items.slice(0, props.limit) : props.items));

const maxValue = computed(() => Math.max(0, ...props.items.map(item => item.value)));

function widthPercent(value: number): string {
  if (maxValue.value <= 0) {
    return '0%';
  }
  return `${Math.max(4, Math.round((value / maxValue.value) * 100))}%`;
}
</script>

<template>
  <div v-if="visible.length > 0">
    <ul class="flex flex-col gap-2">
      <li v-for="item of visible" :key="item.label">
        <component
          :is="selectable ? 'button' : 'div'"
          :type="selectable ? 'button' : undefined"
          class="grid w-full grid-cols-[7.5rem_1fr_3.5rem] items-center gap-2 rounded-md px-1 py-0.5 text-left"
          :class="
            selectable ? 'transition-colors hover:bg-muted/50 focus-visible:bg-muted/50 focus-visible:outline-none' : ''
          "
          @click="selectable ? emit('select', item) : undefined"
        >
          <span class="min-w-0 truncate text-xs text-muted-foreground" :title="item.label">{{ item.label }}</span>
          <span class="block h-2 overflow-hidden rounded-full bg-muted">
            <span class="block h-full rounded-full bg-primary/70" :style="{ width: widthPercent(item.value) }" />
          </span>
          <span class="text-right text-xs tabular-nums">{{ item.value }}</span>
        </component>
      </li>
    </ul>
  </div>
  <div v-else class="flex h-20 items-center justify-center text-sm text-muted-foreground">暂无数据</div>
</template>

<script setup lang="ts">
/**
 * 应用内统一的页面头：标题 + 元信息 + 右侧动作区。
 *
 * 只负责排版层级，不携带页面骨架边距——px/pt 由调用方通过 class 传入，
 * 保证各页面（禅道 / 周报 / …）标题区视觉一致。
 */
defineProps<{
  title: string;
}>();
</script>

<template>
  <header class="flex items-center gap-3">
    <h1 class="text-lg leading-none font-semibold">
      {{ title }}
    </h1>
    <div
      v-if="$slots.meta"
      class="min-w-0 truncate text-xs text-muted-foreground"
    >
      <slot name="meta" />
    </div>
    <div
      v-if="$slots.actions"
      class="ml-auto flex shrink-0 items-center gap-2"
    >
      <slot name="actions" />
    </div>
  </header>
</template>

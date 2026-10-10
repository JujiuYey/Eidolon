<script setup lang="ts">
import { Lightbulb } from 'lucide-vue-next';
import { computed } from 'vue';

import { Badge } from '@/components/ui/badge';
import type { ZentaoStory } from '@/types/zentao';

import {
  formatHours,
  formatIsoDateTime,
  priLabel,
  sortStories,
  storyStageLabel,
  storyStatusLabel,
} from '../utils/display';

const props = defineProps<{
  stories: ZentaoStory[];
}>();

const sorted = computed(() => sortStories(props.stories));

/** 优先级 → 卡片左缘色条颜色（P1 红 / P2 橙 / P3 蓝 / 其余灰） */
function priBarClass(pri: number): string {
  if (pri <= 1) {
    return 'bg-destructive';
  }
  if (pri === 2) {
    return 'bg-orange-500';
  }
  if (pri === 3) {
    return 'bg-sky-500';
  }
  return 'bg-muted-foreground/40';
}
</script>

<template>
  <div class="grid gap-3 md:grid-cols-2">
    <article
      v-for="story of sorted"
      :key="story.id"
      class="group relative overflow-hidden rounded-lg border bg-card p-4 pl-5 transition-colors hover:border-ring/40 hover:bg-muted/30"
    >
      <!-- 优先级色条 -->
      <span class="absolute inset-y-0 left-0 w-1.5" :class="priBarClass(story.pri)" aria-hidden="true" />

      <div class="flex items-start justify-between gap-3">
        <h3 class="line-clamp-2 min-w-0 text-sm leading-6 font-medium">
          <Lightbulb
            class="mr-1.5 inline-block size-4 shrink-0 align-[-2px]"
            :class="story.status === 'active' ? 'text-primary' : 'text-muted-foreground'"
          />
          {{ story.title }}
        </h3>
        <Badge variant="secondary" class="shrink-0">
          {{ priLabel(story.pri) }}
        </Badge>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted-foreground">
        <Badge :variant="story.status === 'active' ? 'default' : 'outline'" class="h-5 px-1.5 text-[11px]">
          {{ storyStatusLabel(story.status) }}
        </Badge>

        <span class="inline-flex items-center gap-1"> 阶段：{{ storyStageLabel(story.stage) }} </span>

        <span v-if="story.estimate > 0" class="tabular-nums">预估 {{ formatHours(story.estimate) }}h</span>
      </div>

      <footer
        v-if="story.product_name || story.opened_by.realname"
        class="mt-3 border-t pt-2.5 text-xs text-muted-foreground"
      >
        <template v-if="story.product_name">
          {{ story.product_name }}
        </template>
        <template v-if="story.product_name && story.opened_by.realname"> · </template>
        <template v-if="story.opened_by.realname">
          {{ story.opened_by.realname }} 提交于 {{ formatIsoDateTime(story.opened_date) }}
        </template>
      </footer>
    </article>
  </div>
</template>

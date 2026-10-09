<script setup lang="ts">
import type { ZentaoBug } from '@/types/zentao';
import { computed } from 'vue';
import { Bug, CalendarClock } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import {
  bugStatusLabel,
  formatIsoDateTime,
  resolutionLabel,
  severityLabel,
  sortBugs,
} from '../utils/display';

const props = defineProps<{
  bugs: ZentaoBug[];
}>();

const sorted = computed(() => sortBugs(props.bugs));

/** 严重级别 → 卡片左缘色条颜色（S1 红 / S2 橙 / S3 蓝 / 其余灰） */
function severityBarClass(severity: number): string {
  if (severity <= 1) {
    return 'bg-destructive';
  }
  if (severity === 2) {
    return 'bg-orange-500';
  }
  if (severity === 3) {
    return 'bg-sky-500';
  }
  return 'bg-muted-foreground/40';
}
</script>

<template>
  <div class="grid gap-3 md:grid-cols-2">
    <article
      v-for="bug of sorted"
      :key="bug.id"
      class="group relative overflow-hidden rounded-lg border bg-card p-4 pl-5 transition-colors hover:border-ring/40 hover:bg-muted/30"
    >
      <!-- 严重级别色条 -->
      <span
        class="absolute inset-y-0 left-0 w-1.5"
        :class="severityBarClass(bug.severity)"
        aria-hidden="true"
      />

      <div class="flex items-start justify-between gap-3">
        <h3 class="line-clamp-2 min-w-0 text-sm leading-6 font-medium">
          <Bug
            class="mr-1.5 inline-block size-4 shrink-0 align-[-2px]"
            :class="bug.status === 'active' ? 'text-destructive' : 'text-muted-foreground'"
          />
          {{ bug.title }}
        </h3>
        <Badge
          variant="secondary"
          class="shrink-0"
        >
          {{ severityLabel(bug.severity) }}
        </Badge>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted-foreground">
        <Badge
          :variant="bug.status === 'active' ? 'destructive' : 'outline'"
          class="h-5 px-1.5 text-[11px]"
        >
          {{ bugStatusLabel(bug.status) }}
        </Badge>

        <span
          v-if="bug.resolution"
          class="inline-flex items-center gap-1"
        >
          已处理：{{ resolutionLabel(bug.resolution) }}
        </span>

        <span
          v-if="bug.deadline"
          class="inline-flex items-center gap-1 tabular-nums"
        >
          <CalendarClock class="size-3.5" />
          {{ bug.deadline }}
        </span>
      </div>

      <footer
        v-if="bug.execution_name || bug.opened_by.realname"
        class="mt-3 border-t pt-2.5 text-xs text-muted-foreground"
      >
        <template v-if="bug.execution_name">
          {{ bug.execution_name }}
        </template>
        <template v-if="bug.execution_name && bug.opened_by.realname">
          ·
        </template>
        <template v-if="bug.opened_by.realname">
          {{ bug.opened_by.realname }} 提交于 {{ formatIsoDateTime(bug.opened_date) }}
        </template>
      </footer>
    </article>
  </div>
</template>

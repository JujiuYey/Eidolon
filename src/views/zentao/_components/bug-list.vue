<script setup lang="ts">
import type { ZentaoBug } from '@/types/zentao';
import { computed } from 'vue';
import { Badge } from '@/components/ui/badge';
import {
  bugStatusLabel,
  bugStatusVariant,
  formatIsoDateTime,
  priLabel,
  priVariant,
  resolutionLabel,
  severityLabel,
  severityVariant,
  sortBugs,
} from '../utils/display';

const props = defineProps<{
  bugs: ZentaoBug[];
}>();

const sorted = computed(() => sortBugs(props.bugs));
</script>

<template>
  <div class="flex flex-col divide-y">
    <div
      v-for="bug of sorted"
      :key="bug.id"
      class="flex flex-wrap items-center gap-x-3 gap-y-1 px-1 py-3 transition-colors hover:bg-muted/50"
    >
      <Badge :variant="severityVariant(bug.severity)">
        {{ severityLabel(bug.severity) }}
      </Badge>

      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium">
          {{ bug.title }}
        </p>
        <p class="truncate text-xs text-muted-foreground">
          {{ bug.execution_name }}<template v-if="bug.opened_by.realname">
            · {{ bug.opened_by.realname }} 提交于 {{ formatIsoDateTime(bug.opened_date) }}
          </template>
        </p>
      </div>

      <span
        v-if="bug.resolution"
        class="text-xs text-muted-foreground"
      >
        {{ resolutionLabel(bug.resolution) }}
      </span>

      <div
        v-if="bug.deadline"
        class="text-xs tabular-nums text-muted-foreground"
      >
        {{ bug.deadline }}
      </div>

      <Badge :variant="priVariant(bug.pri)">
        {{ priLabel(bug.pri) }}
      </Badge>

      <Badge :variant="bugStatusVariant(bug.status)">
        {{ bugStatusLabel(bug.status) }}
      </Badge>
    </div>
  </div>
</template>

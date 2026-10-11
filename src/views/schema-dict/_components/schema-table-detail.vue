<script setup lang="ts">
import { KeyRound, ListTree } from 'lucide-vue-next';

import { Badge } from '@/components/ui/badge';
import type { ParsedColumn, ParsedTable } from '@/types/schema-dict';

import { formatDataType } from '../utils/display';

defineProps<{
  table: ParsedTable | null;
}>();

function columnClass(column: ParsedColumn): string {
  return column.primary_key ? 'font-medium text-foreground' : '';
}
</script>

<template>
  <div v-if="table" class="flex min-h-0 flex-col gap-4">
    <div>
      <h2 class="font-mono text-base font-semibold break-all">{{ table.name }}</h2>
      <p v-if="table.comment" class="mt-1 text-sm text-muted-foreground">{{ table.comment }}</p>
      <div class="mt-2 flex items-center gap-3 text-xs text-muted-foreground">
        <span class="inline-flex items-center gap-1">
          <ListTree class="size-3.5" />
          {{ table.columns.length }} 个字段
        </span>
        <span v-if="table.indexes.length > 0">{{ table.indexes.length }} 个索引</span>
      </div>
    </div>

    <div class="overflow-x-auto rounded-xl border">
      <table class="w-full min-w-[560px] text-left text-sm">
        <thead>
          <tr class="border-b bg-muted/40 text-xs text-muted-foreground">
            <th class="px-3 py-2 font-medium">字段</th>
            <th class="px-3 py-2 font-medium">类型</th>
            <th class="px-3 py-2 font-medium">可空</th>
            <th class="px-3 py-2 font-medium">默认值</th>
            <th class="px-3 py-2 font-medium">注释</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="column of table.columns"
            :key="column.name"
            class="border-b transition-colors last:border-b-0 hover:bg-muted/30"
          >
            <td class="px-3 py-2 align-top">
              <span class="inline-flex items-center gap-1.5 font-mono text-[13px]" :class="columnClass(column)">
                {{ column.name }}
                <KeyRound v-if="column.primary_key" class="size-3.5 text-amber-500" aria-label="主键" />
              </span>
            </td>
            <td class="px-3 py-2 align-top font-mono text-xs text-muted-foreground">
              {{ formatDataType(column.data_type) }}
            </td>
            <td class="px-3 py-2 align-top">
              <Badge v-if="!column.nullable || column.primary_key" variant="secondary" class="text-xs">非空</Badge>
              <span v-else class="text-xs text-muted-foreground">是</span>
            </td>
            <td
              class="max-w-40 truncate px-3 py-2 align-top font-mono text-xs text-muted-foreground"
              :title="column.default ?? ''"
            >
              {{ column.default ?? '-' }}
            </td>
            <td class="max-w-60 px-3 py-2 align-top text-xs text-muted-foreground">{{ column.comment || '-' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="table.indexes.length > 0" class="space-y-2">
      <h3 class="text-sm font-semibold">索引</h3>
      <div class="space-y-1.5">
        <div
          v-for="index of table.indexes"
          :key="`${index.name}-${index.columns.join('-')}`"
          class="flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-xs"
        >
          <Badge v-if="index.unique" variant="secondary" class="text-xs">唯一</Badge>
          <span class="font-mono">{{ index.name || '（无名索引）' }}</span>
          <span class="text-muted-foreground">({{ index.columns.join(', ') }})</span>
        </div>
      </div>
    </div>
  </div>

  <div v-else class="flex flex-1 items-center justify-center">
    <p class="text-sm text-muted-foreground">在左侧选择一张表查看结构</p>
  </div>
</template>

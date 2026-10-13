<script setup lang="ts">
import { Search } from 'lucide-vue-next';
import { computed, ref } from 'vue';

import { Input } from '@/components/ui/input';
import type { ParsedTable } from '@/types/schema-dict';

import { filterTables, sortTables } from '../utils/display';

const props = defineProps<{
  tables: ParsedTable[];
  activeName: string;
}>();

const emit = defineEmits<{
  (e: 'select', table: ParsedTable): void;
}>();

const keyword = ref('');

const visibleTables = computed(() => sortTables(filterTables(props.tables, keyword.value)));
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-2">
    <div class="relative">
      <Search class="absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
      <Input v-model="keyword" class="h-9 rounded-lg pl-8" placeholder="搜索表名或注释" aria-label="搜索表" />
    </div>

    <div class="min-h-0 flex-1 space-y-1 overflow-y-auto pr-1">
      <button
        v-for="table of visibleTables"
        :key="table.name"
        type="button"
        class="w-full rounded-lg border px-3 py-2 text-left transition-colors"
        :class="
          table.name === activeName
            ? 'border-primary/50 bg-primary/5'
            : 'border-transparent hover:border-ring/40 hover:bg-muted/40'
        "
        @click="emit('select', table)"
      >
        <div class="truncate font-mono text-[13px] font-medium">{{ table.name }}</div>
        <div class="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
          <span>{{ table.columns.length }} 字段</span>
          <span v-if="table.comment" class="truncate">{{ table.comment }}</span>
        </div>
      </button>

      <p v-if="visibleTables.length === 0" class="py-8 text-center text-sm text-muted-foreground">
        {{ tables.length === 0 ? '没有表' : '没有匹配的表' }}
      </p>
    </div>
  </div>
</template>

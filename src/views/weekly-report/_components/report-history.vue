<script setup lang="ts">
import type { WeeklyReportEntry } from '@/types/weekly-report';
import { History, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDateTime } from '../utils/report-helpers';

interface Props {
  reports: WeeklyReportEntry[];
  activeId: string | null;
  isLoading: boolean;
}

interface Emits {
  (e: 'select', report: WeeklyReportEntry): void;
  (e: 'remove', reportId: string): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();

const removeOpen = ref(false);
const pendingReport = ref<WeeklyReportEntry | null>(null);

function askRemove(report: WeeklyReportEntry) {
  pendingReport.value = report;
  removeOpen.value = true;
}

function confirmRemove() {
  if (pendingReport.value) {
    emit('remove', pendingReport.value.id);
  }
  removeOpen.value = false;
  pendingReport.value = null;
}
</script>

<template>
  <div class="flex min-h-0 flex-col rounded-xl border bg-card">
    <div class="flex items-center gap-2 px-4 py-3">
      <h2 class="flex items-center gap-2 text-sm font-semibold">
        <History class="h-4 w-4 text-primary" />
        历史周报
      </h2>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-1 px-2 pb-2">
        <template v-if="isLoading">
          <Skeleton
            v-for="index of 3"
            :key="index"
            class="mx-2 h-12"
          />
        </template>

        <p
          v-else-if="reports.length === 0"
          class="px-2 py-6 text-center text-xs text-muted-foreground"
        >
          生成的周报保存后会出现在这里。
        </p>

        <template v-else>
          <button
            v-for="report of reports"
            :key="report.id"
            class="group flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-muted/60"
            :class="report.id === activeId ? 'bg-muted' : ''"
            type="button"
            @click="emit('select', report)"
          >
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium">
                {{ report.title }}
              </p>
              <p class="text-xs text-muted-foreground">
                {{ formatDateTime(report.updated_at) }} · {{ report.commit_count }} 次提交
              </p>
            </div>
            <Button
              aria-label="删除周报"
              class="opacity-0 transition group-hover:opacity-100"
              size="icon"
              variant="ghost"
              @click.stop="askRemove(report)"
            >
              <Trash2 class="h-4 w-4 text-muted-foreground hover:text-destructive" />
            </Button>
          </button>
        </template>
      </div>
    </ScrollArea>

    <SagConfirm
      v-model:open="removeOpen"
      :description="pendingReport
        ? `删除「${pendingReport.title}」后无法恢复。`
        : ''"
      title="删除周报"
      type="destructive"
      @confirm="confirmRemove"
    />
  </div>
</template>

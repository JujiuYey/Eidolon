<script setup lang="ts">
import type { DateRange as CalendarDateRange } from 'reka-ui';
import type { WeeklyReportAuthorMode, WeeklyReportEntry, WeeklyReportRepo } from '@/types/weekly-report';
import { CalendarDate, getLocalTimeZone } from '@internationalized/date';
import { save } from '@tauri-apps/plugin-dialog';
import { CalendarRange, ClipboardCopy, Download, NotebookPen, RefreshCw, Save, WandSparkles } from 'lucide-vue-next';
import { computed, onMounted, shallowRef } from 'vue';
import { toast } from 'vue-sonner';
import SagMarkdownEditor from '@/components/sag/sag-markdown-editor/index.vue';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { RangeCalendar } from '@/components/ui/range-calendar';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import {
  addWeeklyReportRepo,
  deleteWeeklyReport,
  exportWeeklyReport,
  fetchWeeklyReportCommits,
  listWeeklyReports,
  listWeeklyReportRepos,
  polishWeeklyReport,
  removeWeeklyReportRepo,
  saveWeeklyReport,
} from '@/services';
import { copyToClipboard, getErrorMessage } from '@/utils/helpers';
import { useWeeklyReportStore } from '@/stores/weekly-report';
import ReportHistory from './_components/report-history.vue';
import RepoManager from './_components/repo-manager.vue';
import {
  aggregateCommits,
  buildReportTitle,
  collectCommits,
  formatRange,
  getPresetRange,

} from './utils/report-helpers';
import type { DateRange, RangePreset, ReportGroupBy } from './utils/report-helpers';

const DAY_MS = 24 * 60 * 60 * 1000;

const prefs = useWeeklyReportStore();

// ===== 仓库与历史 =====

const repos = ref<WeeklyReportRepo[]>([]);
const isLoadingRepos = ref(false);
const reports = ref<WeeklyReportEntry[]>([]);
const isLoadingReports = ref(false);

// ===== 统计区间与选项 =====

const currentRange = ref<DateRange>(getPresetRange('this_week'));
const activePreset = ref<RangePreset | 'custom'>('this_week');
const isCalendarOpen = ref(false);

const rangeLabel = computed(() => formatRange(currentRange.value));
const reportTitle = computed(() => buildReportTitle(currentRange.value));

const presetOptions: Array<{ key: RangePreset; label: string }> = [
  { key: 'this_week', label: '本周' },
  { key: 'last_week', label: '上周' },
  { key: 'last_7_days', label: '近 7 天' },
  { key: 'last_30_days', label: '近 30 天' },
];

const authorModeOptions: Array<{ key: WeeklyReportAuthorMode; label: string }> = [
  { key: 'auto', label: '只看我的提交' },
  { key: 'custom', label: '自定义作者' },
  { key: 'all', label: '全部作者' },
];

const groupByOptions: Array<{ key: ReportGroupBy; label: string }> = [
  { key: 'type', label: '按类型' },
  { key: 'repo', label: '按仓库' },
];

// ===== 草稿状态 =====

const reportContent = ref('');
const loadedReportId = ref<string | null>(null);
const lastStats = ref<{ repoCount: number; commitCount: number } | null>(null);
const isFetching = ref(false);
const isPolishing = ref(false);

// ===== 日历区间换算 =====

function msToCalendarDate(ms: number): CalendarDate {
  const date = new Date(ms);
  return new CalendarDate(date.getFullYear(), date.getMonth() + 1, date.getDate());
}

/** 日历的受控选中值；点选第一天时允许暂存不完整区间 */
const calendarValue = shallowRef<CalendarDateRange>({
  start: msToCalendarDate(currentRange.value.start),
  end: msToCalendarDate(currentRange.value.end),
});

function syncCalendarValue() {
  calendarValue.value = {
    start: msToCalendarDate(currentRange.value.start),
    end: msToCalendarDate(currentRange.value.end),
  };
}

function handleCalendarUpdate(value: Partial<CalendarDateRange> | undefined) {
  if (!value?.start) {
    return;
  }

  // 先让日历显示已点的第一天；选定第二天才提交区间
  calendarValue.value = { start: value.start, end: value.end ?? value.start };
  if (!value.end) {
    return;
  }

  const zone = getLocalTimeZone();
  const startDay = value.start.toDate(zone);
  const endDay = value.end.toDate(zone);

  currentRange.value = {
    start: startDay.getTime(),
    end: endDay.getTime() + DAY_MS - 1,
  };
  activePreset.value = 'custom';
  isCalendarOpen.value = false;
}

function applyPreset(preset: RangePreset) {
  currentRange.value = getPresetRange(preset);
  activePreset.value = preset;
  syncCalendarValue();
}

// ===== 数据加载 =====

async function loadRepos() {
  isLoadingRepos.value = true;
  try {
    repos.value = await listWeeklyReportRepos();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载仓库列表失败'));
  } finally {
    isLoadingRepos.value = false;
  }
}

async function loadReports() {
  isLoadingReports.value = true;
  try {
    reports.value = await listWeeklyReports();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载历史周报失败'));
  } finally {
    isLoadingReports.value = false;
  }
}

// ===== 仓库操作 =====

async function handleAddRepos(paths: string[]) {
  let added = 0;
  for (const path of paths) {
    try {
      await addWeeklyReportRepo(path);
      added += 1;
    } catch (error) {
      toast.error(getErrorMessage(error, `添加仓库失败: ${path}`));
    }
  }
  if (added > 0) {
    toast.success(`已添加 ${added} 个仓库`);
    await loadRepos();
  }
}

async function handleRemoveRepo(repoId: string) {
  try {
    await removeWeeklyReportRepo(repoId);
    toast.success('已移除仓库');
    await loadRepos();
  } catch (error) {
    toast.error(getErrorMessage(error, '移除仓库失败'));
  }
}

// ===== 生成 / 润色 / 保存 =====

async function generate() {
  if (repos.value.length === 0) {
    toast.warning('请先在左侧添加要统计的代码仓库');
    return;
  }
  if (prefs.authorMode === 'custom' && !prefs.customAuthor.trim()) {
    toast.warning('已选择自定义作者，请填写作者姓名或邮箱');
    return;
  }
  if (isFetching.value) {
    return;
  }

  isFetching.value = true;
  try {
    const logs = await fetchWeeklyReportCommits({
      repo_ids: [],
      since_ms: currentRange.value.start,
      until_ms: currentRange.value.end,
      author_mode: prefs.authorMode,
      author: prefs.customAuthor.trim(),
    });

    const collected = collectCommits(logs);
    lastStats.value = { repoCount: collected.repo_count, commitCount: collected.commits.length };

    const markdown = aggregateCommits(logs, { range: currentRange.value, groupBy: prefs.groupBy });
    reportContent.value = markdown;
    loadedReportId.value = null;

    if (collected.commits.length === 0 && collected.errors.length === 0) {
      toast.info('所选区间内没有匹配的提交');
    } else {
      toast.success('周报草稿已生成，可在下方编辑');
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '生成周报失败'));
  } finally {
    isFetching.value = false;
  }
}

async function polish() {
  if (!reportContent.value.trim()) {
    toast.warning('请先生成周报草稿');
    return;
  }
  if (isPolishing.value) {
    return;
  }

  isPolishing.value = true;
  const draftBeforePolish = reportContent.value;
  try {
    const result = await polishWeeklyReport({
      title: reportTitle.value,
      draft: reportContent.value,
    });

    reportContent.value = result.content;
    toast.success(`已使用 ${result.model_label} 润色`, {
      action: {
        label: '撤销',
        onClick: () => {
          reportContent.value = draftBeforePolish;
        },
      },
      duration: 10_000,
    });
  } catch (error) {
    toast.error(getErrorMessage(error, 'AI 润色失败'));
  } finally {
    isPolishing.value = false;
  }
}

async function saveReport() {
  if (!reportContent.value.trim()) {
    toast.warning('还没有可保存的周报内容');
    return;
  }

  try {
    const saved = await saveWeeklyReport({
      id: loadedReportId.value ?? '',
      title: reportTitle.value,
      range_start: currentRange.value.start,
      range_end: currentRange.value.end,
      content: reportContent.value,
      repo_count: lastStats.value?.repoCount ?? 0,
      commit_count: lastStats.value?.commitCount ?? 0,
      created_at: 0,
      updated_at: 0,
    });

    loadedReportId.value = saved.id;
    toast.success('周报已保存');
    await loadReports();
  } catch (error) {
    toast.error(getErrorMessage(error, '保存周报失败'));
  }
}

async function exportReport() {
  if (!reportContent.value.trim()) {
    toast.warning('还没有可导出的周报内容');
    return;
  }

  const target = await save({
    title: '导出周报',
    defaultPath: `${reportTitle.value}.md`,
    filters: [{ name: 'Markdown', extensions: ['md'] }],
  });
  if (!target) {
    return;
  }

  try {
    await exportWeeklyReport(target, reportContent.value);
    toast.success(`已导出到 ${target}`);
  } catch (error) {
    toast.error(getErrorMessage(error, '导出周报失败'));
  }
}

async function copyReport() {
  if (!reportContent.value.trim()) {
    toast.warning('还没有可复制的周报内容');
    return;
  }
  const ok = await copyToClipboard(reportContent.value);
  if (ok) {
    toast.success('周报已复制到剪贴板');
  }
}

// ===== 历史操作 =====

function selectReport(report: WeeklyReportEntry) {
  loadedReportId.value = report.id;
  reportContent.value = report.content;
  currentRange.value = { start: report.range_start, end: report.range_end };
  activePreset.value = 'custom';
  syncCalendarValue();
  lastStats.value = { repoCount: report.repo_count, commitCount: report.commit_count };
}

async function removeReport(reportId: string) {
  try {
    await deleteWeeklyReport(reportId);
    if (loadedReportId.value === reportId) {
      loadedReportId.value = null;
    }
    toast.success('已删除周报');
    await loadReports();
  } catch (error) {
    toast.error(getErrorMessage(error, '删除周报失败'));
  }
}

onMounted(() => {
  void loadRepos();
  void loadReports();
});
</script>

<template>
  <div class="mx-auto flex h-screen max-w-7xl flex-col overflow-hidden p-6">
    <div class="mb-4 shrink-0">
      <h1 class="mb-1 flex items-center gap-2 text-2xl font-bold">
        <NotebookPen class="h-6 w-6 text-primary" />
        周报生成
      </h1>
      <p class="text-muted-foreground">
        汇总本地仓库的 git 提交记录，一键生成、润色并导出每周工作周报。
      </p>
    </div>

    <div class="flex min-h-0 flex-1 gap-4">
      <!-- 左栏：仓库 + 历史 -->
      <aside class="flex w-72 shrink-0 flex-col gap-4">
        <RepoManager
          class="min-h-0 flex-1"
          :is-loading="isLoadingRepos"
          :repos="repos"
          @add="handleAddRepos"
          @remove="handleRemoveRepo"
        />
        <ReportHistory
          class="min-h-0 flex-1"
          :active-id="loadedReportId"
          :is-loading="isLoadingReports"
          :reports="reports"
          @remove="removeReport"
          @select="selectReport"
        />
      </aside>

      <!-- 右侧：工具条 + 编辑器 -->
      <div class="flex min-w-0 flex-1 flex-col gap-4">
        <div class="shrink-0 space-y-3 rounded-xl border bg-card p-4">
          <div class="flex flex-wrap items-center gap-2">
            <Button
              v-for="option of presetOptions"
              :key="option.key"
              size="sm"
              :variant="activePreset === option.key ? 'default' : 'outline'"
              @click="applyPreset(option.key)"
            >
              {{ option.label }}
            </Button>

            <Popover v-model:open="isCalendarOpen">
              <PopoverTrigger as-child>
                <Button
                  size="sm"
                  :variant="activePreset === 'custom' ? 'default' : 'outline'"
                >
                  <CalendarRange class="h-4 w-4" />
                  {{ rangeLabel }}
                </Button>
              </PopoverTrigger>
              <PopoverContent
                align="start"
                class="w-auto p-0"
              >
                <RangeCalendar
                  :number-of-months="1"
                  :model-value="calendarValue"
                  @update:model-value="handleCalendarUpdate"
                />
              </PopoverContent>
            </Popover>

            <Button
              class="ml-auto"
              :disabled="isFetching"
              size="sm"
              @click="generate"
            >
              <RefreshCw
                class="h-4 w-4"
                :class="isFetching ? 'animate-spin' : ''"
              />
              {{ isFetching ? '拉取中…' : '生成周报' }}
            </Button>
          </div>

          <Separator />

          <div class="flex flex-wrap items-center gap-2">
            <Select
              :model-value="prefs.authorMode"
              @update:model-value="value => prefs.authorMode = String(value ?? '') as WeeklyReportAuthorMode"
            >
              <SelectTrigger class="h-8 w-[10.5rem] text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option of authorModeOptions"
                  :key="option.key"
                  :value="option.key"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <input
              v-if="prefs.authorMode === 'custom'"
              v-model="prefs.customAuthor"
              class="border-input placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 h-8 w-44 rounded-md border bg-transparent px-2 text-xs shadow-xs outline-none focus-visible:ring-[3px]"
              placeholder="作者姓名或邮箱"
            />

            <div class="flex items-center overflow-hidden rounded-md border">
              <Button
                v-for="option of groupByOptions"
                :key="option.key"
                class="rounded-none border-0"
                size="sm"
                :variant="prefs.groupBy === option.key ? 'secondary' : 'ghost'"
                @click="prefs.groupBy = option.key"
              >
                {{ option.label }}
              </Button>
            </div>

            <div class="ml-auto flex items-center gap-2">
              <Button
                :disabled="isPolishing || !reportContent.trim()"
                size="sm"
                variant="outline"
                @click="polish"
              >
                <WandSparkles
                  class="h-4 w-4"
                  :class="isPolishing ? 'animate-pulse' : ''"
                />
                {{ isPolishing ? '润色中…' : 'AI 润色' }}
              </Button>
              <Button
                :disabled="!reportContent.trim()"
                size="sm"
                variant="outline"
                @click="saveReport"
              >
                <Save class="h-4 w-4" />
                保存
              </Button>
              <Button
                :disabled="!reportContent.trim()"
                size="sm"
                variant="outline"
                @click="copyReport"
              >
                <ClipboardCopy class="h-4 w-4" />
                复制
              </Button>
              <Button
                :disabled="!reportContent.trim()"
                size="sm"
                variant="outline"
                @click="exportReport"
              >
                <Download class="h-4 w-4" />
                导出
              </Button>
            </div>
          </div>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto rounded-xl border bg-card p-4">
          <SagMarkdownEditor
            v-model="reportContent"
            badge-label="周报正文"
            editor-hint="生成后可自由修改，保存后进入左侧历史列表。"
            editor-label="周报内容"
            placeholder="点击「生成周报」，或直接在这里撰写周报内容。"
            title="支持 Markdown，编辑预览同步进行。"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { refDebounced } from '@vueuse/core';
import { Search } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';

import SagBarChart from '@/components/sag/sag-bar-chart/index.vue';
import SagDonutChart from '@/components/sag/sag-donut-chart/index.vue';
import type { DonutSegment } from '@/components/sag/sag-donut-chart/index.vue';
import SagHBarList from '@/components/sag/sag-h-bar-list/index.vue';
import SagStatsStrip from '@/components/sag/sag-stats-strip/index.vue';
import type { StatTile } from '@/components/sag/sag-stats-strip/index.vue';
import { Input } from '@/components/ui/input';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useMailStore } from '@/stores/mail';
import type { SentEmail } from '@/types/mail';
import { dailyLabels } from '@/utils/stats';

import { computeMailStats, accountSendCounts, lastSentHint } from '../utils/mail-stats';
import ComposePanel from './ComposePanel.vue';
import SentDisplay from './SentDisplay.vue';
import SentList from './SentList.vue';
import TemplateManagerDialog from './TemplateManagerDialog.vue';
import TemplatePicker from './TemplatePicker.vue';

const store = useMailStore();
const router = useRouter();

const templateManagerOpen = ref(false);
const searchValue = ref('');
const debouncedSearch = refDebounced(searchValue, 250);

const filteredSentList = computed(() => {
  const keyword = debouncedSearch.value?.trim().toLowerCase();
  if (!keyword) {
    return store.sentEmails;
  }

  return store.sentEmails.filter(
    (email: SentEmail) =>
      email.subject.toLowerCase().includes(keyword) ||
      email.to_addresses.toLowerCase().includes(keyword) ||
      email.account_email.toLowerCase().includes(keyword) ||
      email.body.toLowerCase().includes(keyword),
  );
});

const mailStats = computed(() => computeMailStats(store.templates, store.sentEmails));
const statTiles = computed<StatTile[]>(() => [
  { label: '模板', value: mailStats.value.templateCount },
  { label: '已发送', value: mailStats.value.sentCount, hint: lastSentHint(mailStats.value.lastSentAt) },
  { label: '本周发送', value: mailStats.value.weekSentCount },
  {
    label: '发送失败',
    value: mailStats.value.failedCount,
    tone: mailStats.value.failedCount > 0 ? 'danger' : 'default',
  },
]);
const sendValues = computed(() => mailStats.value.dailySent);
const sendLabels = computed(() => dailyLabels(sendValues.value.length));

const statusSegments = computed<DonutSegment[]>(() => {
  const failed = mailStats.value.failedCount;
  return [
    { label: '发送成功', value: Math.max(0, mailStats.value.sentCount - failed), tone: 'emerald' },
    { label: '发送失败', value: failed, tone: 'red' },
  ];
});

const accountBarItems = computed(() => accountSendCounts(store.sentEmails));

function handleAccountChange(value: unknown) {
  if (typeof value === 'string') {
    store.selectedAccountId = value;
  }
}

function handleViewChange(value: string | number) {
  if (value === 'compose' || value === 'sent') {
    store.viewMode = value;
  }
}

/** 账户管理统一在应用设置，写信页只负责选择 */
function goSettings() {
  router.push('/app-setting');
}

function handleResend() {
  const mail = store.selectedSentEmail;
  if (mail) {
    store.resend(mail);
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <header class="flex shrink-0 items-center gap-3 px-6 pt-6">
      <h1 class="text-lg leading-none font-semibold">邮件</h1>

      <div class="ml-auto flex items-center gap-2">
        <Select
          v-if="store.accounts.length > 0"
          :model-value="store.selectedAccountId"
          @update:model-value="handleAccountChange"
        >
          <SelectTrigger class="w-56">
            <SelectValue placeholder="选择发件账户" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="account of store.accounts" :key="account.id" :value="account.id">
              {{ account.name }}（{{ account.email }}）
            </SelectItem>
          </SelectContent>
        </Select>
        <button
          v-else
          type="button"
          class="text-xs text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
          @click="goSettings"
        >
          请先在设置中配置发件账户
        </button>

        <Tabs :model-value="store.viewMode" @update:model-value="handleViewChange">
          <TabsList>
            <TabsTrigger value="compose"> 写信 </TabsTrigger>
            <TabsTrigger value="sent">
              已发送
              <span v-if="store.sentEmails.length > 0" class="ml-1 text-xs text-muted-foreground tabular-nums">
                {{ store.sentEmails.length }}
              </span>
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>
    </header>

    <Separator class="mx-6 mt-4 shrink-0" />

    <div v-if="store.templates.length > 0 || store.sentEmails.length > 0" class="mx-6 mt-4 shrink-0 space-y-4">
      <SagStatsStrip :tiles="statTiles" />

      <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <div class="rounded-xl border bg-card p-4 md:col-span-2">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-medium">近 30 天发送</h3>
            <span class="text-xs text-muted-foreground">按天</span>
          </div>
          <div class="mt-3">
            <SagBarChart
              :values="sendValues"
              :labels="sendLabels"
              :tooltips="mailStats.dailyTooltips"
              aria-label="近 30 天发送柱状图"
            />
          </div>
        </div>

        <div class="rounded-xl border bg-card p-4">
          <h3 class="text-sm font-medium">发送结果分布</h3>
          <div class="mt-3">
            <SagDonutChart :segments="statusSegments" center-label="封邮件" aria-label="发送结果分布环形图" />
          </div>
        </div>

        <div class="flex flex-col rounded-xl border bg-card p-4">
          <h3 class="text-sm font-medium">各账户发送量</h3>
          <div class="mt-3 min-h-0 flex-1">
            <SagHBarList :items="accountBarItems" :limit="6" />
          </div>
        </div>
      </div>
    </div>

    <ResizablePanelGroup id="mail-panel-group" direction="horizontal" class="min-h-0 flex-1">
      <!-- 左栏：写信时选模板，已发送时列历史 -->
      <ResizablePanel id="mail-list-panel" class="flex h-full min-h-0 flex-col" :default-size="34" :min-size="25">
        <TemplatePicker
          v-if="store.viewMode === 'compose'"
          v-model:selected-template-id="store.selectedTemplateId"
          class="min-h-0 flex-1"
          :templates="store.templates"
          @manage="templateManagerOpen = true"
        />

        <template v-else>
          <div class="shrink-0 px-4 pt-4 pb-3">
            <div class="relative">
              <Search class="absolute top-2.5 left-2.5 size-4 text-muted-foreground" />
              <Input v-model="searchValue" placeholder="搜索主题、收件人或正文" class="pl-8" />
            </div>
          </div>
          <SentList v-model:selected-id="store.selectedSentEmailId" class="min-h-0 flex-1" :items="filteredSentList" />
        </template>
      </ResizablePanel>
      <ResizableHandle id="mail-display-handle" with-handle />

      <!-- 右栏：写信表单或发送详情 -->
      <ResizablePanel id="mail-display-panel" :default-size="66">
        <ComposePanel v-if="store.viewMode === 'compose'" />
        <SentDisplay v-else :mail="store.selectedSentEmail" @resend="handleResend" />
      </ResizablePanel>
    </ResizablePanelGroup>

    <TemplateManagerDialog v-model:open="templateManagerOpen" />
  </div>
</template>

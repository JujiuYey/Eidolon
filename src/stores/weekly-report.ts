import { defineStore } from 'pinia';
import { ref } from 'vue';

import type { WeeklyReportAuthorMode } from '@/types/weekly-report';
import type { ReportGroupBy } from '@/views/weekly-report/utils/report-helpers';

const AUTHOR_MODES: readonly WeeklyReportAuthorMode[] = ['mine', 'custom', 'all'];

/**
 * 归一化作者过滤方式。历史版本持久化过 'auto'（按各仓库 git config 过滤），
 * 该模式已由「设置 → Git 账号」邮箱匹配取代，遗留值恢复为默认的 'mine'。
 */
export function normalizeAuthorMode(value: unknown): WeeklyReportAuthorMode {
  return AUTHOR_MODES.includes(value as WeeklyReportAuthorMode) ? (value as WeeklyReportAuthorMode) : 'mine';
}

/**
 * 周报页面的 UI 偏好。仅持久化轻量设置；
 * 仓库列表与周报历史走后端 SQLite。
 */
export const useWeeklyReportStore = defineStore(
  'weekly-report',
  () => {
    /** 作者过滤方式：mine = 按设置的 Git 账号邮箱匹配，custom = 指定作者，all = 不过滤 */
    const authorMode = ref<WeeklyReportAuthorMode>('mine');
    /** custom 模式下的作者（匹配姓名或邮箱） */
    const customAuthor = ref('');
    /** 草稿分组方式 */
    const groupBy = ref<ReportGroupBy>('type');

    return { authorMode, customAuthor, groupBy };
  },
  {
    persist: {
      key: 'eidolon-weekly-report-prefs',
      pick: ['authorMode', 'customAuthor', 'groupBy'],
      afterHydrate: context => {
        context.store.authorMode = normalizeAuthorMode(context.store.authorMode);
      },
    },
  },
);

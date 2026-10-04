import type { WeeklyReportAuthorMode } from '@/types/weekly-report';
import type { ReportGroupBy } from '@/views/weekly-report/utils/report-helpers';
import { defineStore } from 'pinia';
import { ref } from 'vue';

/**
 * 周报页面的 UI 偏好。仅持久化轻量设置；
 * 仓库列表与周报历史走后端 SQLite。
 */
export const useWeeklyReportStore = defineStore('weekly-report', () => {
  /** 作者过滤方式：auto = 各仓库 git 配置，custom = 指定作者，all = 不过滤 */
  const authorMode = ref<WeeklyReportAuthorMode>('auto');
  /** custom 模式下的作者（匹配姓名或邮箱） */
  const customAuthor = ref('');
  /** 草稿分组方式 */
  const groupBy = ref<ReportGroupBy>('type');

  return { authorMode, customAuthor, groupBy };
}, {
  persist: {
    key: 'eidolon-weekly-report-prefs',
    pick: ['authorMode', 'customAuthor', 'groupBy'],
  },
});

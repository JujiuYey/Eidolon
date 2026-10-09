/**
 * 禅道页展示辅助函数（纯函数，可单测）。
 *
 * 状态/优先级文案与徽章变体、逾期判断、工时与时间格式化、列表排序。
 */

import type { ZentaoBug, ZentaoTask } from '@/types/zentao';
import type { BadgeVariants } from '@/components/ui/badge';

export type BadgeVariant = NonNullable<BadgeVariants['variant']>;

/** 任务状态 → 中文文案 */
export const TASK_STATUS_LABELS: Record<string, string> = {
  wait: '未开始',
  doing: '进行中',
  pause: '已暂停',
  done: '已完成',
  closed: '已关闭',
  cancel: '已取消',
};

/** Bug 状态 → 中文文案 */
export const BUG_STATUS_LABELS: Record<string, string> = {
  active: '激活',
  resolved: '已解决',
  closed: '已关闭',
  delay: '延期',
};

/** Bug 解决方案 → 中文文案 */
export const RESOLUTION_LABELS: Record<string, string> = {
  '': '',
  'bydesign': '设计如此',
  'duplicate': '重复 Bug',
  'external': '外部原因',
  'fixed': '已解决',
  'notrepro': '无法重现',
  'postponed': '延后处理',
  'willnotfix': '不予解决',
};

export function taskStatusLabel(status: string): string {
  return TASK_STATUS_LABELS[status] ?? status;
}

export function bugStatusLabel(status: string): string {
  return BUG_STATUS_LABELS[status] ?? status;
}

export function resolutionLabel(resolution: string): string {
  return RESOLUTION_LABELS[resolution] ?? resolution;
}

/** 优先级徽章：P1 最紧急。0 视为未设置 */
export function priLabel(pri: number): string {
  return pri > 0 ? `P${pri}` : '-';
}

export function priVariant(pri: number): BadgeVariant {
  if (pri <= 1) {
    return 'destructive';
  }
  if (pri === 2) {
    return 'default';
  }
  if (pri === 3) {
    return 'secondary';
  }
  return 'outline';
}

/** 严重级别：1 最严重 */
export function severityLabel(severity: number): string {
  return severity > 0 ? `S${severity}` : '-';
}

export function severityVariant(severity: number): BadgeVariant {
  if (severity <= 1) {
    return 'destructive';
  }
  if (severity === 2) {
    return 'default';
  }
  if (severity === 3) {
    return 'secondary';
  }
  return 'outline';
}

/** 状态徽章样式：进行中/激活高亮，其余弱化 */
export function taskStatusVariant(status: string): BadgeVariant {
  return status === 'doing' ? 'default' : 'secondary';
}

export function bugStatusVariant(status: string): BadgeVariant {
  return status === 'active' ? 'destructive' : 'secondary';
}

/** deadline 形如 2026-09-18；早于今天视为逾期（仅对未完成任务调用） */
export function isOverdue(deadline: string, today = new Date()): boolean {
  if (!deadline) {
    return false;
  }
  const parsed = parseLocalDate(deadline);
  if (!parsed) {
    return false;
  }
  const todayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  return parsed.getTime() < todayStart.getTime();
}

function parseLocalDate(text: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text.trim());
  if (!match) {
    return null;
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(year, month - 1, day);
  return Number.isNaN(date.getTime()) ? null : date;
}

/** 工时显示：0 或小数位多余则精简 */
export function formatHours(hours: number): string {
  if (!Number.isFinite(hours) || hours === 0) {
    return '-';
  }
  const rounded = Math.round(hours * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

/** ISO 时间串 → 本地短格式（如 2026/09/14 16:46）；无效输入原样返回 */
export function formatIsoDateTime(iso: string): string {
  if (!iso) {
    return '-';
  }
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** Unix 毫秒 → 本地短格式 */
export function formatTimestamp(ms: number): string {
  if (!ms) {
    return '-';
  }
  return formatIsoDateTime(new Date(ms).toISOString());
}

/** 任务排序：优先级升序 → 有截止日期且更早在前 → id 升序 */
export function sortTasks(tasks: ZentaoTask[]): ZentaoTask[] {
  return [...tasks].sort((a, b) => {
    if (a.pri !== b.pri) {
      return a.pri - b.pri;
    }
    const deadlineOrder = compareDeadline(a.deadline, b.deadline);
    if (deadlineOrder !== 0) {
      return deadlineOrder;
    }
    return a.id - b.id;
  });
}

/** Bug 排序：严重级别升序 → 优先级升序 → id 升序 */
export function sortBugs(bugs: ZentaoBug[]): ZentaoBug[] {
  return [...bugs].sort((a, b) => {
    if (a.severity !== b.severity) {
      return a.severity - b.severity;
    }
    if (a.pri !== b.pri) {
      return a.pri - b.pri;
    }
    return a.id - b.id;
  });
}

function compareDeadline(a: string, b: string): number {
  if (!a && !b) {
    return 0;
  }
  if (!a) {
    return 1;
  }
  if (!b) {
    return -1;
  }
  return a.localeCompare(b);
}

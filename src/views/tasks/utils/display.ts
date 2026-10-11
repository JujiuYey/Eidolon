/**
 * 任务页展示辅助函数（纯函数，可单测）。
 *
 * 状态/优先级文案与色点、截止日期判断（本地时区）、列表排序与过滤计数。
 * 截止日期统一为 'YYYY-MM-DD' 文本，字典序即时间序。
 */

import type { Task, TaskPriority, TaskStatus } from '@/types/tasks';
import { countSince, dailyCounts, dailyTooltips, DAY_MS, dateString, startOfDay, weekStartMs } from '@/utils/stats';

/** 任务状态 → 中文文案 */
export const TASK_STATUS_LABELS: Record<TaskStatus, string> = {
  todo: '待办',
  doing: '进行中',
  done: '已完成',
};

/** 优先级 → 展示文案（对齐禅道 P1/P2/P3 语言） */
export const TASK_PRIORITY_LABELS: Record<TaskPriority, string> = {
  high: 'P1',
  medium: 'P2',
  low: 'P3',
  none: '无',
};

export function taskStatusLabel(status: TaskStatus): string {
  return TASK_STATUS_LABELS[status] ?? status;
}

export function taskPriorityLabel(priority: TaskPriority): string {
  return TASK_PRIORITY_LABELS[priority] ?? priority;
}

/** 优先级 → 色点颜色（P1 红 / P2 橙 / P3 蓝，与禅道页色条一致） */
export function priorityDotClass(priority: TaskPriority): string {
  if (priority === 'high') {
    return 'bg-destructive';
  }
  if (priority === 'medium') {
    return 'bg-orange-500';
  }
  if (priority === 'low') {
    return 'bg-sky-500';
  }
  return 'bg-muted-foreground/40';
}

/** 本地时区的今天，'YYYY-MM-DD' */
export function todayDateString(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

function isIsoDate(text: string): boolean {
  return /^(\d{4})-(\d{2})-(\d{2})$/.test(text.trim());
}

/** 早于今天视为逾期（仅对未完成任务调用） */
export function isOverdue(dueDate: string, today = todayDateString()): boolean {
  return isIsoDate(dueDate) && dueDate.trim() < today;
}

export function isDueToday(dueDate: string, today = todayDateString()): boolean {
  return isIsoDate(dueDate) && dueDate.trim() === today;
}

const PRIORITY_RANK: Record<TaskPriority, number> = { high: 0, medium: 1, low: 2, none: 3 };
const NO_DUE = '9999-12-31';

/** 任务排序：未完成在前（截止早 → 优先级高 → 创建早），已完成按完成时间倒序垫底 */
export function sortTasks(tasks: Task[]): Task[] {
  const open = tasks
    .filter(task => task.status !== 'done')
    .sort((a, b) => {
      const dueOrder = (a.due_date || NO_DUE).localeCompare(b.due_date || NO_DUE);
      if (dueOrder !== 0) {
        return dueOrder;
      }
      const priorityOrder = PRIORITY_RANK[a.priority] - PRIORITY_RANK[b.priority];
      if (priorityOrder !== 0) {
        return priorityOrder;
      }
      return a.created_at - b.created_at;
    });

  const done = tasks
    .filter(task => task.status === 'done')
    .sort((a, b) => (b.completed_at ?? 0) - (a.completed_at ?? 0));

  return [...open, ...done];
}

export type TaskFilter = 'all' | TaskStatus;

/** 各过滤项的任务数 */
export function filterCounts(tasks: Task[]): Record<TaskFilter, number> {
  const counts: Record<TaskFilter, number> = { all: tasks.length, todo: 0, doing: 0, done: 0 };
  for (const task of tasks) {
    if (task.status in counts) {
      counts[task.status] += 1;
    }
  }
  return counts;
}

/** 按过滤项取任务（all 保持原序，交给 sortTasks 统一排序） */
export function filterTasks(tasks: Task[], filter: TaskFilter): Task[] {
  if (filter === 'all') {
    return tasks;
  }
  return tasks.filter(task => task.status === filter);
}

/** 任务统计瓦片 + 完成趋势 */
export interface TaskStats {
  overdueCount: number;
  weekCompletedCount: number;
  monthCompletedCount: number;
  totalCompletedCount: number;
  dailyCompleted: number[];
  dailyTooltips: string[];
}

/** 从任务列表计算统计：逾期、按周/30 天/累计完成、30 天完成分布 */
export function computeTaskStats(tasks: Task[], now = Date.now()): TaskStats {
  const completedTimestamps = tasks
    .filter(task => task.status === 'done' && task.completed_at)
    .map(task => task.completed_at as number);
  const daily = dailyCounts(completedTimestamps, 30, now);

  return {
    overdueCount: tasks.filter(task => task.status !== 'done' && isOverdue(task.due_date, dateString(now))).length,
    weekCompletedCount: countSince(completedTimestamps, weekStartMs(now)),
    monthCompletedCount: countSince(completedTimestamps, startOfDay(now) - 29 * DAY_MS),
    totalCompletedCount: completedTimestamps.length,
    dailyCompleted: daily,
    dailyTooltips: dailyTooltips(daily, '个完成', now),
  };
}

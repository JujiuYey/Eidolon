import { describe, expect, it } from 'vitest';

import type { Task } from '@/types/tasks';

import {
  computeTaskStats,
  filterCounts,
  filterTasks,
  isDueToday,
  isOverdue,
  priorityDotClass,
  sortTasks,
  taskPriorityLabel,
  taskStatusLabel,
  todayDateString,
} from '../display';

const NOW = new Date(2026, 9, 14, 15).getTime(); // 周三下午
const DAY = 24 * 60 * 60 * 1000;

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task_1',
    title: '写周报',
    notes: '',
    status: 'todo',
    priority: 'none',
    due_date: '',
    start_at: '',
    end_at: '',
    estimate_minutes: 0,
    source: 'manual',
    sort: 0,
    completed_at: null,
    created_at: 100,
    updated_at: 100,
    ...overrides,
  };
}

describe('task display helpers', () => {
  it('labels statuses and priorities', () => {
    expect(taskStatusLabel('todo')).toBe('待办');
    expect(taskStatusLabel('doing')).toBe('进行中');
    expect(taskStatusLabel('done')).toBe('已完成');
    expect(taskPriorityLabel('high')).toBe('P1');
    expect(taskPriorityLabel('none')).toBe('无');
  });

  it('maps priority to dot colors aligned with zentao bars', () => {
    expect(priorityDotClass('high')).toBe('bg-destructive');
    expect(priorityDotClass('medium')).toBe('bg-orange-500');
    expect(priorityDotClass('low')).toBe('bg-sky-500');
    expect(priorityDotClass('none')).toBe('bg-muted-foreground/40');
  });

  it('formats today as local zero-padded ISO date', () => {
    expect(todayDateString()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('judges overdue and due-today against local today', () => {
    const today = '2026-10-11';
    expect(isOverdue('2026-10-10', today)).toBe(true);
    expect(isOverdue('2026-10-11', today)).toBe(false);
    expect(isOverdue('2026-10-12', today)).toBe(false);
    expect(isOverdue('', today)).toBe(false);
    expect(isOverdue('周五', today)).toBe(false);
    expect(isDueToday('2026-10-11', today)).toBe(true);
    expect(isDueToday('2026-10-12', today)).toBe(false);
  });

  it('sorts open tasks by due date then priority then created_at, done tasks last', () => {
    const tasks = [
      makeTask({ id: 'a', due_date: '', priority: 'high' }),
      makeTask({ id: 'b', status: 'done', completed_at: 50, due_date: '2026-10-01' }),
      makeTask({ id: 'c', due_date: '2026-10-12', priority: 'low' }),
      makeTask({ id: 'd', due_date: '2026-10-12', priority: 'high' }),
      makeTask({ id: 'e', due_date: '2026-10-11', priority: 'none' }),
      makeTask({ id: 'f', status: 'doing', due_date: '', priority: 'high' }),
      makeTask({ id: 'g', status: 'done', completed_at: 200 }),
    ];

    const sorted = sortTasks(tasks);
    expect(sorted.map(task => task.id)).toEqual(['e', 'd', 'c', 'a', 'f', 'g', 'b']);
  });

  it('counts tasks per filter and filters by status', () => {
    const tasks = [
      makeTask({ id: 'a' }),
      makeTask({ id: 'b', status: 'doing' }),
      makeTask({ id: 'c', status: 'done' }),
    ];

    expect(filterCounts(tasks)).toEqual({ all: 3, todo: 1, doing: 1, done: 1 });
    expect(filterTasks(tasks, 'doing').map(task => task.id)).toEqual(['b']);
    expect(filterTasks(tasks, 'all')).toHaveLength(3);
  });

  it('computes task stats: overdue, completion counts and daily trend', () => {
    const tasks = [
      makeTask({ id: 'overdue', due_date: '2026-10-10' }), // 未完成且逾期
      makeTask({ id: 'due-today', due_date: '2026-10-14' }),
      makeTask({ id: 'done-today', status: 'done', completed_at: NOW }),
      makeTask({ id: 'done-yesterday', status: 'done', completed_at: NOW - 1 * DAY }),
      makeTask({ id: 'done-old', status: 'done', completed_at: NOW - 40 * DAY }),
      makeTask({ id: 'done-no-timestamp', status: 'done', completed_at: null }),
    ];

    const stats = computeTaskStats(tasks, NOW);

    expect(stats.overdueCount).toBe(1); // 今天到期的不算逾期
    expect(stats.weekCompletedCount).toBe(2); // 本周完成：今天 + 昨天
    expect(stats.monthCompletedCount).toBe(2);
    expect(stats.totalCompletedCount).toBe(3); // 无完成时间戳的不计
    expect(stats.dailyCompleted).toHaveLength(30);
    expect(stats.dailyCompleted[29]).toBe(1);
    expect(stats.dailyCompleted[0]).toBe(0); // 40 天前在窗口外
    expect(stats.dailyTooltips[29]).toContain('1 个完成');
  });
});

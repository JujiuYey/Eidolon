import { describe, expect, it } from 'vitest';
import type { ZentaoBug, ZentaoTask } from '@/types/zentao';
import {
  bugStatusLabel,
  formatHours,
  formatIsoDateTime,
  formatTimestamp,
  isOverdue,
  priLabel,
  resolutionLabel,
  severityLabel,
  sortBugs,
  sortTasks,
  taskStatusLabel,
} from '../display';

function buildTask(overrides: Partial<ZentaoTask> = {}): ZentaoTask {
  return {
    id: 1,
    name: '任务',
    pri: 3,
    status: 'doing',
    assigned_to: { account: 'hjc', realname: '胡建成' },
    execution: 7,
    execution_name: '迭代一',
    deadline: '',
    estimate: 8,
    left: 4,
    consumed: 4,
    story_title: '',
    task_type: 'devel',
    ...overrides,
  };
}

function buildBug(overrides: Partial<ZentaoBug> = {}): ZentaoBug {
  return {
    id: 1,
    title: 'Bug',
    severity: 2,
    pri: 2,
    status: 'active',
    assigned_to: { account: 'hjc', realname: '胡建成' },
    resolution: '',
    opened_by: { account: 'lwb', realname: '李四' },
    opened_date: '2026-09-14T08:46:52Z',
    deadline: '',
    execution: 7,
    execution_name: '迭代一',
    ...overrides,
  };
}

describe('zentao display labels', () => {
  it('maps task statuses to chinese labels', () => {
    expect(taskStatusLabel('wait')).toBe('未开始');
    expect(taskStatusLabel('doing')).toBe('进行中');
    expect(taskStatusLabel('pause')).toBe('已暂停');
    expect(taskStatusLabel('unknown')).toBe('unknown');
  });

  it('maps bug statuses and resolutions', () => {
    expect(bugStatusLabel('active')).toBe('激活');
    expect(bugStatusLabel('delay')).toBe('延期');
    expect(resolutionLabel('willnotfix')).toBe('不予解决');
    expect(resolutionLabel('')).toBe('');
    expect(resolutionLabel('custom')).toBe('custom');
  });

  it('formats priority and severity', () => {
    expect(priLabel(0)).toBe('-');
    expect(priLabel(1)).toBe('P1');
    expect(severityLabel(0)).toBe('-');
    expect(severityLabel(3)).toBe('S3');
  });

  it('formats hours', () => {
    expect(formatHours(0)).toBe('-');
    expect(formatHours(8)).toBe('8');
    expect(formatHours(4.5)).toBe('4.5');
    expect(formatHours(4.25)).toBe('4.3');
  });

  it('formats timestamps', () => {
    expect(formatIsoDateTime('')).toBe('-');
    expect(formatIsoDateTime('not-a-date')).toBe('not-a-date');
    // UTC 午夜在东八区是 08:00
    expect(formatIsoDateTime('2026-09-14T00:00:00Z')).toContain('2026/09/14');
    expect(formatTimestamp(0)).toBe('-');
    expect(formatTimestamp(1_789_500_000_000)).toContain('2026');
  });
});

describe('isOverdue', () => {
  it('treats empty or malformed deadline as not overdue', () => {
    expect(isOverdue('')).toBe(false);
    expect(isOverdue('2026/09/18')).toBe(false);
  });

  it('marks past dates as overdue and today as not overdue', () => {
    // 2026-09-18
    const today = new Date(2026, 8, 18);
    expect(isOverdue('2026-09-17', today)).toBe(true);
    expect(isOverdue('2026-09-18', today)).toBe(false);
    expect(isOverdue('2026-09-19', today)).toBe(false);
  });
});

describe('sortTasks / sortBugs', () => {
  it('sorts tasks by priority then deadline then id', () => {
    const tasks = [
      buildTask({ id: 3, pri: 1, deadline: '2026-10-01' }),
      buildTask({ id: 2, pri: 2, deadline: '2026-09-20' }),
      buildTask({ id: 9, pri: 2, deadline: '2026-09-18' }),
      buildTask({ id: 4, pri: 2, deadline: '' }),
    ];
    const sorted = sortTasks(tasks);
    expect(sorted.map(task => task.id)).toEqual([3, 9, 2, 4]);
  });

  it('does not mutate the input array', () => {
    const tasks = [buildTask({ id: 2, pri: 2 }), buildTask({ id: 1, pri: 1 })];
    const sorted = sortTasks(tasks);
    expect(tasks[0]?.id).toBe(2);
    expect(sorted[0]?.id).toBe(1);
  });

  it('sorts bugs by severity then priority', () => {
    const bugs = [
      buildBug({ id: 1, severity: 3, pri: 1 }),
      buildBug({ id: 2, severity: 2, pri: 3 }),
      buildBug({ id: 3, severity: 2, pri: 2 }),
    ];
    const sorted = sortBugs(bugs);
    expect(sorted.map(bug => bug.id)).toEqual([3, 2, 1]);
  });
});

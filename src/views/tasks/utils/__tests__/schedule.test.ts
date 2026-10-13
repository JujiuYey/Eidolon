import { describe, expect, it } from 'vitest';

import type { Task } from '@/types/tasks';

import {
  addDays,
  buildDatetime,
  computeLanes,
  dateKey,
  durationToHeight,
  formatMinutes,
  minutesToOffset,
  parseDatetime,
  parseTimeToMinutes,
  scheduleDuration,
  snapMinutes,
  taskToBlock,
  weekDays,
} from '../schedule';

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
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

describe('时间解析与格式化', () => {
  it('parseTimeToMinutes 接受 HH:MM 并拒绝越界', () => {
    expect(parseTimeToMinutes('09:00')).toBe(540);
    expect(parseTimeToMinutes('24:00')).toBe(1440);
    expect(parseTimeToMinutes('23:59')).toBe(1439);
    expect(parseTimeToMinutes('24:01')).toBeNull();
    expect(parseTimeToMinutes('9点')).toBeNull();
    expect(parseTimeToMinutes('25:00')).toBeNull();
  });

  it('formatMinutes 与 parseTimeToMinutes 往返一致', () => {
    expect(formatMinutes(540)).toBe('09:00');
    expect(formatMinutes(1440)).toBe('24:00');
    expect(formatMinutes(0)).toBe('00:00');
    for (const text of ['00:00', '08:30', '13:45', '23:59']) {
      expect(formatMinutes(parseTimeToMinutes(text)!)).toBe(text);
    }
  });

  it('parseDatetime / buildDatetime 往返一致，坏值返回 null', () => {
    expect(parseDatetime('2026-10-14T09:00')).toEqual({ date: '2026-10-14', minutes: 540 });
    expect(parseDatetime('2026-10-14 09:00')).toBeNull();
    expect(parseDatetime('')).toBeNull();
    expect(buildDatetime('2026-10-14', 540)).toBe('2026-10-14T09:00');
  });

  it('snapMinutes 向下吸附并夹在一天内', () => {
    expect(snapMinutes(542)).toBe(540);
    expect(snapMinutes(553)).toBe(540);
    expect(snapMinutes(555)).toBe(555);
    expect(snapMinutes(-20)).toBe(0);
    expect(snapMinutes(2000)).toBe(1440);
  });
});

describe('网格几何', () => {
  it('分钟与像素互算', () => {
    expect(minutesToOffset(0)).toBe(0);
    expect(minutesToOffset(60)).toBe(56);
    expect(minutesToOffset(90)).toBe(84);
    expect(durationToHeight(60)).toBe(56);
    expect(durationToHeight(5)).toBe(14); // 最短按 15 分钟渲染
  });
});

describe('任务与时间块的映射', () => {
  it('未排期任务映射为 null', () => {
    expect(taskToBlock(makeTask(), '2026-10-13')).toBeNull();
  });

  it('已排期任务映射为块，逾期标记跟随截止日', () => {
    const task = makeTask({
      start_at: '2026-10-14T09:00',
      end_at: '2026-10-14T10:30',
      due_date: '2026-10-12',
      priority: 'high',
    });
    const block = taskToBlock(task, '2026-10-13')!;
    expect(block.date).toBe('2026-10-14');
    expect(block.start).toBe(540);
    expect(block.end).toBe(630);
    expect(block.overdue).toBe(true);

    const dueToday = taskToBlock({ ...task, due_date: '2026-10-13' }, '2026-10-13')!;
    // 截止日当天不算逾期（对齐清单视图 isOverdue）
    expect(dueToday.overdue).toBe(false);

    const notOverdue = taskToBlock({ ...task, due_date: '2026-10-20' }, '2026-10-13')!;
    expect(notOverdue.overdue).toBe(false);

    const doneTask = taskToBlock({ ...task, status: 'done' }, '2026-10-13')!;
    expect(doneTask.overdue).toBe(false);
  });

  it('坏排期（跨日/时序颠倒）映射为 null', () => {
    expect(
      taskToBlock(makeTask({ start_at: '2026-10-14T09:00', end_at: '2026-10-15T09:00' }), '2026-10-13'),
    ).toBeNull();
    expect(
      taskToBlock(makeTask({ start_at: '2026-10-14T11:00', end_at: '2026-10-14T10:00' }), '2026-10-13'),
    ).toBeNull();
  });

  it('scheduleDuration 只在同日内计算', () => {
    expect(scheduleDuration(makeTask({ start_at: '2026-10-14T09:00', end_at: '2026-10-14T10:30' }))).toBe(90);
    expect(scheduleDuration(makeTask())).toBe(0);
  });
});

describe('周与日期', () => {
  it('weekDays 从周一排到周日', () => {
    // 2026-10-13 是周二，周一应为 10-12
    const days = weekDays(new Date(2026, 9, 13));
    expect(days).toHaveLength(7);
    expect(dateKey(days[0]!)).toBe('2026-10-12');
    expect(dateKey(days[6]!)).toBe('2026-10-18');
    expect(days[0]!.getDay()).toBe(1);
  });

  it('addDays 跨月正确', () => {
    expect(dateKey(addDays(new Date(2026, 9, 30), 1))).toBe('2026-10-31');
    expect(dateKey(addDays(new Date(2026, 9, 31), 1))).toBe('2026-11-01');
  });
});

describe('重叠布局泳道', () => {
  function block(id: string, start: number, end: number) {
    return {
      id,
      date: '2026-10-14',
      start,
      end,
      title: id,
      status: 'todo' as const,
      priority: 'none' as const,
      overdue: false,
    };
  }

  it('互不重叠的块各占整列', () => {
    const lanes = computeLanes([block('a', 540, 600), block('b', 600, 660), block('c', 700, 760)]);
    expect(lanes.get('a')).toEqual({ lane: 0, lanes: 1 });
    expect(lanes.get('b')).toEqual({ lane: 0, lanes: 1 });
    expect(lanes.get('c')).toEqual({ lane: 0, lanes: 1 });
  });

  it('成簇重叠的块并排分泳道，簇结束后重置', () => {
    // a 与 b、c 均重叠，b 与 c 互相重叠（570–630 与 600–660），三块各占一泳道
    const lanes = computeLanes([
      block('a', 540, 660),
      block('b', 570, 630),
      block('c', 600, 660),
      block('d', 700, 760),
    ]);
    expect(lanes.get('a')).toEqual({ lane: 0, lanes: 3 });
    expect(lanes.get('b')).toEqual({ lane: 1, lanes: 3 });
    expect(lanes.get('c')).toEqual({ lane: 2, lanes: 3 });
    expect(lanes.get('d')).toEqual({ lane: 0, lanes: 1 }); // 新簇重新从整列开始
  });

  it('两块部分重叠时占两泳道', () => {
    const lanes = computeLanes([block('a', 540, 660), block('b', 600, 720)]);
    expect(lanes.get('a')).toEqual({ lane: 0, lanes: 2 });
    expect(lanes.get('b')).toEqual({ lane: 1, lanes: 2 });
  });

  it('三块全重叠时占三泳道', () => {
    const lanes = computeLanes([block('a', 540, 660), block('b', 540, 660), block('c', 540, 660)]);
    expect(lanes.get('a')).toEqual({ lane: 0, lanes: 3 });
    expect(lanes.get('b')).toEqual({ lane: 1, lanes: 3 });
    expect(lanes.get('c')).toEqual({ lane: 2, lanes: 3 });
  });

  it('空列表与首尾相接的块', () => {
    expect(computeLanes([]).size).toBe(0);
    // 540 结束与 540 开始不算重叠
    const lanes = computeLanes([block('a', 480, 540), block('b', 540, 600)]);
    expect(lanes.get('a')!.lanes).toBe(1);
    expect(lanes.get('b')!.lanes).toBe(1);
  });
});

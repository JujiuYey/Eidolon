/**
 * 日程视图的时间纯函数（可单测）。
 *
 * 时间块统一用「日期 + 分钟数」表达：date 是 'YYYY-MM-DD'，start/end 是当天 0 点起的分钟数（0–1440）。
 * 持久化格式是任务上的 start_at/end_at（本地无时区 ISO，'YYYY-MM-DDTHH:MM'），两端不做时区换算。
 * 网格几何：每小时 56px，15 分钟吸附，与设计稿一致。
 */

import type { Task, TaskPriority, TaskStatus } from '@/types/tasks';

export const MINUTES_PER_DAY = 24 * 60;
export const SNAP_MINUTES = 15;
export const PX_PER_HOUR = 56;
export const GUTTER_WIDTH = 44;

/** 网格内容总高度（px） */
export const GRID_HEIGHT = (MINUTES_PER_DAY / 60) * PX_PER_HOUR;

/** 网格内渲染的时间块（由任务映射而来） */
export interface TimeBlock {
  id: string;
  /** 所属日 'YYYY-MM-DD' */
  date: string;
  /** 当天 0 点起的分钟数 */
  start: number;
  end: number;
  title: string;
  status: TaskStatus;
  priority: TaskPriority;
  /** 未完成且已过截止日 */
  overdue: boolean;
}

/** 'HH:MM' → 分钟数；非法返回 null（24:00 允许，表示一天结束） */
export function parseTimeToMinutes(text: string): number | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(text.trim());
  if (!match) {
    return null;
  }
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 24 || minutes > 59 || (hours === 24 && minutes !== 0)) {
    return null;
  }
  return hours * 60 + minutes;
}

/** 分钟数 → 'HH:MM'（1440 → '24:00'） */
export function formatMinutes(minutes: number): string {
  const clamped = Math.max(0, Math.min(MINUTES_PER_DAY, Math.round(minutes)));
  const hours = Math.floor(clamped / 60);
  const rest = clamped % 60;
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${pad(hours)}:${pad(rest)}`;
}

/** 'YYYY-MM-DDTHH:MM' → { date, minutes }；非法返回 null */
export function parseDatetime(value: string): { date: string; minutes: number } | null {
  const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})$/.exec(value.trim());
  if (!match) {
    return null;
  }
  const minutes = parseTimeToMinutes(match[2]!);
  if (minutes === null) {
    return null;
  }
  return { date: match[1]!, minutes };
}

/** { date, minutes } → 'YYYY-MM-DDTHH:MM' */
export function buildDatetime(date: string, minutes: number): string {
  return `${date}T${formatMinutes(minutes)}`;
}

/** 向下吸附到 15 分钟（或指定步长），结果限制在一天内 */
export function snapMinutes(minutes: number, snap = SNAP_MINUTES): number {
  const clamped = Math.max(0, Math.min(MINUTES_PER_DAY, minutes));
  return Math.floor(clamped / snap) * snap;
}

/** 分钟 → 网格 top（px） */
export function minutesToOffset(minutes: number): number {
  return (minutes / 60) * PX_PER_HOUR;
}

/** 时长（分钟） → 块高度（px），最小 15 分钟起 */
export function durationToHeight(duration: number): number {
  return (Math.max(SNAP_MINUTES, duration) / 60) * PX_PER_HOUR;
}

/** 任务是否已排期 */
export function hasSchedule(task: Task): boolean {
  return task.start_at.trim() !== '' && task.end_at.trim() !== '';
}

/** 排期时长（分钟）；未排期为 0 */
export function scheduleDuration(task: Task): number {
  if (!hasSchedule(task)) {
    return 0;
  }
  const start = parseDatetime(task.start_at);
  const end = parseDatetime(task.end_at);
  if (!start || !end || start.date !== end.date) {
    return 0;
  }
  return Math.max(0, end.minutes - start.minutes);
}

/** 任务 → 时间块；未排期或数据异常返回 null */
export function taskToBlock(task: Task, today: string): TimeBlock | null {
  if (!hasSchedule(task)) {
    return null;
  }
  const start = parseDatetime(task.start_at);
  const end = parseDatetime(task.end_at);
  if (!start || !end || start.date !== end.date || end.minutes <= start.minutes) {
    return null;
  }
  return {
    id: task.id,
    date: start.date,
    start: start.minutes,
    end: end.minutes,
    title: task.title,
    status: task.status,
    priority: task.priority,
    overdue: task.status !== 'done' && task.due_date.trim() !== '' && task.due_date.trim() < today,
  };
}

/** 一周（周一起）7 个 Date；入参是本周任意一天 */
export function weekDays(anyDay: Date): Date[] {
  const weekday = (anyDay.getDay() + 6) % 7; // 周一=0 … 周日=6
  const monday = addDays(anyDay, -weekday);
  return Array.from({ length: 7 }, (_, index) => addDays(monday, index));
}

export function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

/** Date → 'YYYY-MM-DD'（本地时区） */
export function dateKey(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** 两个日期键是否同一天 */
export function isSameDay(a: string, b: string): boolean {
  return a === b;
}

/** 重叠布局：给每个块分配 { lane, lanes }，同簇并排、不同簇互不影响 */
export function computeLanes(blocks: TimeBlock[]): Map<string, { lane: number; lanes: number }> {
  const result = new Map<string, { lane: number; lanes: number }>();
  const sorted = [...blocks].sort((a, b) => a.start - b.start || b.end - a.end);

  let cluster: TimeBlock[] = [];
  let clusterEnd = -1;

  const flush = () => {
    if (cluster.length === 0) {
      return;
    }
    // 贪心分配泳道：找第一个末尾不冲突的泳道
    const laneEnds: number[] = [];
    const assignment: Array<{ block: TimeBlock; lane: number }> = [];
    for (const block of cluster) {
      let lane = laneEnds.findIndex(end => end <= block.start);
      if (lane === -1) {
        lane = laneEnds.length;
        laneEnds.push(block.end);
      } else {
        laneEnds[lane] = block.end;
      }
      assignment.push({ block, lane });
    }
    const lanes = laneEnds.length;
    for (const { block, lane } of assignment) {
      result.set(block.id, { lane, lanes });
    }
    cluster = [];
  };

  for (const block of sorted) {
    if (cluster.length > 0 && block.start >= clusterEnd) {
      flush();
      clusterEnd = -1;
    }
    cluster.push(block);
    clusterEnd = Math.max(clusterEnd, block.end);
  }
  flush();

  return result;
}

/** 块的显示时长文本：'45 分钟' / '1 小时' / '1.5 小时' */
export function durationLabel(minutes: number): string {
  if (minutes < 60) {
    return `${minutes} 分钟`;
  }
  if (minutes % 60 === 0) {
    return `${minutes / 60} 小时`;
  }
  return `${(minutes / 60).toFixed(1)} 小时`;
}

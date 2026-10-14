/**
 * 日程网格的日列几何与落点吸附（纯函数，可单测）。
 *
 * 时间↔像素的换算沿用 schedule.ts 的 minutesToOffset / durationToHeight，
 * 这里只补 canvas 版特有的两件事：
 * 1. 日列几何（表头偏移 + 每个日列的 x 区间），集中一处避免日偏移算错；
 * 2. 落点/缩放的合法区间钳制。
 *
 * 坐标系：x 自左向右（左侧 GUTTER_WIDTH 是时间轴标签列），
 * y 自上向下（顶部 HEADER_HEIGHT 是日期表头，再往下才是 0:00–24:00 网格）。
 */

import {
  durationToHeight,
  GUTTER_WIDTH,
  MINUTES_PER_DAY,
  PX_PER_HOUR,
  SNAP_MINUTES,
  snapMinutes,
} from '@/views/tasks/utils/schedule';

/** 顶部日期表头高度 */
export const HEADER_HEIGHT = 40;

/** 单个日列的最小宽度，窄于它时外层出现横向滚动 */
export const MIN_DAY_WIDTH = 92;

/** 表头 + 网格主体的总高度 */
export const CHART_HEIGHT = HEADER_HEIGHT + (MINUTES_PER_DAY / 60) * PX_PER_HOUR;

/** 每个日列的宽度；日列宽是日偏移的换算基准，多处必须同口径 */
export function dayWidth(chartW: number, dayCount: number): number {
  return (chartW - GUTTER_WIDTH) / dayCount;
}

/** 某个日列的左边界 */
export function dayLeft(dayIndex: number, dayW: number): number {
  return GUTTER_WIDTH + dayIndex * dayW;
}

/** 分钟数 → 网格 y（已含表头偏移） */
export function minutesToY(minutes: number): number {
  return HEADER_HEIGHT + (minutes / 60) * PX_PER_HOUR;
}

/** 网格 y → 分钟数（不吸附，可超出 0–1440） */
export function yToMinutes(y: number): number {
  return ((y - HEADER_HEIGHT) / PX_PER_HOUR) * 60;
}

/** x → 日序号，超出范围钳到最近一列 */
export function xToDayIndex(x: number, dayW: number, dayCount: number): number {
  return Math.max(0, Math.min(dayCount - 1, Math.floor((x - GUTTER_WIDTH) / dayW)));
}

/**
 * 拖动落点：吸附到最近的 15 分钟，并把块顶夹在一天之内，避免末尾溢出。
 */
export function snapDrop(y: number, duration: number): { start: number; end: number } {
  const span = Math.max(SNAP_MINUTES, duration);
  const start = Math.max(0, Math.min(snapMinutes(yToMinutes(y)), MINUTES_PER_DAY - span));
  return { start, end: start + span };
}

/** 缩放底边：不得早于块顶 + 15 分钟，不得晚于当天结束 */
export function resizeEnd(y: number, start: number): number {
  return Math.max(start + SNAP_MINUTES, Math.min(MINUTES_PER_DAY, snapMinutes(yToMinutes(y))));
}

/** 缩放顶边：不得晚于块底 - 15 分钟，不得早于 0 点 */
export function resizeStart(y: number, end: number): number {
  return Math.min(end - SNAP_MINUTES, Math.max(0, snapMinutes(yToMinutes(y))));
}

/** 块在画布上的矩形；日偏移与泳道并排都在这里一次算对 */
export interface BlockRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function blockRect(
  dayIndex: number,
  lane: number,
  lanes: number,
  dayW: number,
  start: number,
  end: number,
): BlockRect {
  const slot = dayW / lanes;
  return {
    x: dayLeft(dayIndex, dayW) + lane * slot,
    y: minutesToY(start),
    width: slot,
    height: durationToHeight(end - start),
  };
}

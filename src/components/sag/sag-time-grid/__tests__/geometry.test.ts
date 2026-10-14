import { describe, expect, it } from 'vitest';

import { GUTTER_WIDTH, MINUTES_PER_DAY, PX_PER_HOUR, SNAP_MINUTES } from '@/views/tasks/utils/schedule';

import {
  blockRect,
  CHART_HEIGHT,
  dayLeft,
  dayWidth,
  HEADER_HEIGHT,
  minutesToY,
  resizeEnd,
  resizeStart,
  snapDrop,
  xToDayIndex,
  yToMinutes,
} from '../geometry';

const DAY_COUNT = 7;
const CHART_W = 1000;
const DAY_W = dayWidth(CHART_W, DAY_COUNT);

describe('day column geometry', () => {
  it('splits usable width evenly after the time gutter', () => {
    expect(DAY_W * DAY_COUNT + GUTTER_WIDTH).toBeCloseTo(CHART_W, 6);
  });

  it('anchors each day column after the gutter so blocks never straddle a day', () => {
    // 旧实现漏了日偏移，块会横跨整周；这里逐列断言左边界与列宽
    const boxes = [0, 1, 2, 3, 4, 5, 6].map(index => blockRect(index, 0, 1, DAY_W, 60, 120));
    expect(boxes.map(box => box.x)).toEqual(boxes.map((_, index) => dayLeft(index, DAY_W)));
    expect(boxes.every(box => box.x >= GUTTER_WIDTH)).toBe(true);
    // 列与列首尾相接，不重叠也不留缝（消除浮点尾差）
    const gaps = boxes
      .slice(1)
      .map((box, index) => Number((box.x - (boxes[index]!.x + boxes[index]!.width)).toFixed(6)));
    expect(gaps).toEqual([0, 0, 0, 0, 0, 0]);
    // 单列块的宽度必须正好是一天，不得跨到别列
    expect(boxes[3]!.width).toBeCloseTo(DAY_W, 6);
  });

  it('splits a day into lanes for overlapping blocks', () => {
    const left = blockRect(2, 0, 2, DAY_W, 0, 60);
    const right = blockRect(2, 1, 2, DAY_W, 0, 60);
    expect(right.x - left.x).toBeCloseTo(DAY_W / 2, 6);
    expect(left.width).toBeCloseTo(right.width, 6);
  });

  it('maps x to day index and clamps to the edges', () => {
    expect(xToDayIndex(GUTTER_WIDTH + 1, DAY_W, DAY_COUNT)).toBe(0);
    expect(xToDayIndex(dayLeft(3, DAY_W) + 1, DAY_W, DAY_COUNT)).toBe(3);
    expect(xToDayIndex(-500, DAY_W, DAY_COUNT)).toBe(0);
    expect(xToDayIndex(99999, DAY_W, DAY_COUNT)).toBe(DAY_COUNT - 1);
    // 单日视图永远落在第 0 列
    expect(xToDayIndex(dayLeft(0, DAY_W) + 5, DAY_W, 1)).toBe(0);
  });
});

describe('time to pixel conversion', () => {
  it('offsets the grid below the header row', () => {
    expect(minutesToY(0)).toBe(HEADER_HEIGHT);
    expect(minutesToY(60)).toBeCloseTo(HEADER_HEIGHT + PX_PER_HOUR, 6);
    expect(minutesToY(MINUTES_PER_DAY)).toBeCloseTo(CHART_HEIGHT, 6);
  });

  it('round-trips y back to minutes', () => {
    for (const minutes of [0, 15, 90, 735, MINUTES_PER_DAY]) {
      expect(yToMinutes(minutesToY(minutes))).toBeCloseTo(minutes, 6);
    }
  });

  it('yToMinutes allows values outside the day so callers can clamp explicitly', () => {
    expect(yToMinutes(HEADER_HEIGHT - 100)).toBeLessThan(0);
    expect(yToMinutes(CHART_HEIGHT + 100)).toBeGreaterThan(MINUTES_PER_DAY);
  });
});

describe('drop snapping', () => {
  it('snaps to 15 minutes and keeps the duration', () => {
    const { start, end } = snapDrop(minutesToY(10 * 60 + 7), 60);
    expect(start).toBe(Math.round((10 * 60 + 7) / SNAP_MINUTES) * SNAP_MINUTES);
    expect(end - start).toBe(60);
  });

  it('pushes a block that would overflow midnight back inside the day', () => {
    const { start, end } = snapDrop(minutesToY(MINUTES_PER_DAY - 10), 60);
    expect(end).toBeLessThanOrEqual(MINUTES_PER_DAY);
    expect(start).toBe(MINUTES_PER_DAY - 60);
  });

  it('treats a sub-15-minute duration as 15', () => {
    const { start, end } = snapDrop(minutesToY(600), 5);
    expect(end - start).toBe(SNAP_MINUTES);
  });
});

describe('resize clamping', () => {
  it('keeps at least one snap between the edges', () => {
    expect(resizeEnd(minutesToY(600), 600)).toBe(600 + SNAP_MINUTES);
    expect(resizeStart(minutesToY(600), 600)).toBe(600 - SNAP_MINUTES);
  });

  it('does not cross the day boundary', () => {
    expect(resizeEnd(CHART_HEIGHT + 500, 600)).toBe(MINUTES_PER_DAY);
    expect(resizeStart(-500, 600)).toBe(0);
  });

  it('snaps the dragged edge down to the grid, matching schedule.snapMinutes', () => {
    // 548 分向下吸附到 540，不进位到 555——沿用既有 snapMinutes 语义
    expect(resizeEnd(minutesToY(9 * 60 + 8), 480)).toBe(540);
    expect(resizeStart(minutesToY(9 * 60 + 8), 700)).toBe(540);
  });
});

import { describe, expect, it } from 'vitest';

import {
  consecutiveDays,
  countSince,
  dailyCounts,
  dailyLabels,
  dailyTooltips,
  dateString,
  formatShortDate,
  startOfDay,
  weekStartMs,
} from '../stats';

/** 2026-10-14 是周三；本地时区构造当天某时刻 */
function at(hour: number, minute = 0): number {
  return new Date(2026, 9, 14, hour, minute).getTime();
}

describe('stats date helpers', () => {
  it('startOfDay floors to local midnight', () => {
    expect(startOfDay(at(8, 30))).toBe(new Date(2026, 9, 14, 0, 0, 0, 0).getTime());
  });

  it('dateString formats local ISO date', () => {
    expect(dateString(at(8, 30))).toBe('2026-10-14');
  });

  it('weekStartMs returns this Monday midnight', () => {
    const start = weekStartMs(at(15));
    expect(dateString(start)).toBe('2026-10-12'); // 周三所在周的周一
    expect(new Date(start).getDay()).toBe(1);
  });

  it('countSince counts timestamps on and after the boundary', () => {
    const timestamps = [at(1), at(9), at(23)];
    expect(countSince(timestamps, at(9))).toBe(2);
    expect(countSince(timestamps, at(23))).toBe(1);
    expect(countSince(timestamps, at(0) + 24 * 60 * 60 * 1000)).toBe(0);
  });

  it('dailyCounts buckets into the last N days including today', () => {
    const now = at(15);
    const timestamps = [
      now, // 今天
      now - 1 * 24 * 60 * 60 * 1000, // 昨天
      now - 1 * 24 * 60 * 60 * 1000 + 1000, // 昨天另一条
      now - 3 * 24 * 60 * 60 * 1000, // 3 天前
      now - 40 * 24 * 60 * 60 * 1000, // 窗口外
      now + 2 * 24 * 60 * 60 * 1000, // 未来，忽略
    ];

    const counts = dailyCounts(timestamps, 7, now);
    expect(counts).toHaveLength(7);
    expect(counts[6]).toBe(1); // 今天
    expect(counts[5]).toBe(2); // 昨天
    expect(counts[3]).toBe(1); // 3 天前
    expect(counts[0]).toBe(0);
  });

  it('dailyCounts treats late-night and early-morning as the same local day', () => {
    const now = at(15);
    const lateNightBefore = new Date(2026, 9, 13, 23, 59).getTime();
    const counts = dailyCounts([lateNightBefore], 3, now);
    expect(counts[1]).toBe(1); // index 1 = 昨天（index 2 = 今天）
  });

  it('dailyLabels aligns with dailyCounts buckets', () => {
    const now = at(15);
    const labels = dailyLabels(3, now);
    expect(labels).toEqual(['10月12日', '10月13日', '10月14日']);
  });

  it('dailyTooltips pairs labels with counts', () => {
    const tooltips = dailyTooltips([0, 2, 5], '条收藏', at(15));
    expect(tooltips).toEqual(['10月12日 · 0 条收藏', '10月13日 · 2 条收藏', '10月14日 · 5 条收藏']);
  });

  it('consecutiveDays counts back from today, falling back to yesterday', () => {
    const now = at(15);
    const day = 24 * 60 * 60 * 1000;

    // 今天 + 前两天 → 连续 3 天
    expect(consecutiveDays([now, now - day, now - 2 * day], now)).toBe(3);

    // 今天没记录，昨天 + 前天 → 连续 2 天
    expect(consecutiveDays([now - day, now - 2 * day], now)).toBe(2);

    // 只有 3 天前 → 连续 0 天
    expect(consecutiveDays([now - 3 * day], now)).toBe(0);

    // 同一天多条只算一天
    expect(consecutiveDays([now, now + 3600_000], now)).toBe(1);
  });

  it('formatShortDate pads month and day, blank for zero', () => {
    expect(formatShortDate(at(8, 5))).toBe('10-14');
    expect(formatShortDate(new Date(2026, 0, 3).getTime())).toBe('01-03');
    expect(formatShortDate(0)).toBe('—');
  });
});

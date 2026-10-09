import { describe, expect, it } from 'vitest';
import { formatRelativeTime } from '../format';

const NOW = new Date('2026-10-09T15:00:00').getTime();

describe('formatRelativeTime', () => {
  it('一分钟内显示刚刚', () => {
    expect(formatRelativeTime(NOW - 30_000, NOW)).toBe('刚刚');
    expect(formatRelativeTime(NOW, NOW)).toBe('刚刚');
  });

  it('一小时内显示分钟前', () => {
    expect(formatRelativeTime(NOW - 5 * 60_000, NOW)).toBe('5 分钟前');
    expect(formatRelativeTime(NOW - 59 * 60_000, NOW)).toBe('59 分钟前');
  });

  it('当天更早显示小时前', () => {
    expect(formatRelativeTime(new Date('2026-10-09T09:00:00').getTime(), NOW)).toBe('6 小时前');
  });

  it('昨天显示昨天', () => {
    expect(formatRelativeTime(new Date('2026-10-08T15:00:00').getTime(), NOW)).toBe('昨天');
  });

  it('更早的同一年显示月日', () => {
    expect(formatRelativeTime(new Date('2026-03-01T10:00:00').getTime(), NOW)).toBe('3月1日');
  });

  it('跨年显示带年份', () => {
    expect(formatRelativeTime(new Date('2025-12-31T10:00:00').getTime(), NOW)).toBe('2025年12月31日');
  });

  it('零时间戳返回空字符串', () => {
    expect(formatRelativeTime(0, NOW)).toBe('');
  });
});

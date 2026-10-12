/**
 * 跨页面统计的日期工具（纯函数，可单测）。
 *
 * 统一按本地时区的「自然日」分桶：一天 = 本地零点起 24 小时；
 * 周起始为周一。收藏/任务/邮件等页面的统计瓦片与趋势条共用。
 */

export const DAY_MS = 24 * 60 * 60 * 1000;

/** 本地时区的时间戳归到所属自然日的零点 */
export function startOfDay(ms: number): number {
  const date = new Date(ms);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** 本地时区的 'YYYY-MM-DD' */
export function dateString(ms: number): string {
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** 本地时区本周周一的零点 */
export function weekStartMs(now = Date.now()): number {
  const today = startOfDay(now);
  const weekday = (new Date(today).getDay() + 6) % 7; // 周一=0 … 周日=6
  return today - weekday * DAY_MS;
}

/** 统计时间戳里落在 [sinceMs, +∞) 的数量 */
export function countSince(timestamps: number[], sinceMs: number): number {
  return timestamps.filter(timestamp => timestamp >= sinceMs).length;
}

/**
 * 最近 days 天（含今天）的逐日数量。
 * 返回数组 index 0 = days-1 天前，末位 = 今天；窗口外（含未来）的时间戳忽略。
 */
export function dailyCounts(timestamps: number[], days: number, now = Date.now()): number[] {
  const today = startOfDay(now);
  const buckets = Array.from<number>({ length: days }).fill(0);
  for (const timestamp of timestamps) {
    const offset = Math.floor((startOfDay(timestamp) - today) / DAY_MS);
    if (offset > 0 || offset <= -days) {
      continue;
    }
    const index = offset + days - 1;
    buckets[index] = (buckets[index] ?? 0) + 1;
  }
  return buckets;
}

/** 与 dailyCounts 同参数对齐的日期标签「M月D日」 */
export function dailyLabels(days: number, now = Date.now()): string[] {
  const today = startOfDay(now);
  return Array.from({ length: days }, (_, index) => {
    const date = new Date(today + (index - (days - 1)) * DAY_MS);
    return `${date.getMonth() + 1}月${date.getDate()}日`;
  });
}

/** 趋势条悬浮文案：与 counts 对齐的「M月D日 · N 单位」 */
export function dailyTooltips(counts: number[], unit: string, now = Date.now()): string[] {
  return dailyLabels(counts.length, now).map((label, index) => `${label} · ${counts[index] ?? 0} ${unit}`);
}

/**
 * 最近 months 个月（含本月）的逐月数量。
 * 返回数组 index 0 = 最旧的月份；未来时间戳忽略。
 */
export function monthlyCounts(timestamps: number[], months: number, now = Date.now()): number[] {
  const nowDate = new Date(now);
  const monthIndex = (date: Date) => date.getFullYear() * 12 + date.getMonth();
  const current = monthIndex(nowDate);
  const buckets = Array.from<number>({ length: months }).fill(0);
  for (const timestamp of timestamps) {
    const offset = current - monthIndex(new Date(timestamp));
    if (offset < 0 || offset >= months) {
      continue;
    }
    buckets[months - 1 - offset] = (buckets[months - 1 - offset] ?? 0) + 1;
  }
  return buckets;
}

/** 与 monthlyCounts 对齐的「M月」标签 */
export function monthlyLabels(months: number, now = Date.now()): string[] {
  const nowDate = new Date(now);
  return Array.from({ length: months }, (_, index) => {
    const date = new Date(nowDate.getFullYear(), nowDate.getMonth() - (months - 1 - index), 1);
    return `${date.getMonth() + 1}月`;
  });
}

/** 与 monthlyCounts 对齐的悬浮文案「YYYY年M月 · N 单位」 */
export function monthlyTooltips(counts: number[], unit: string, now = Date.now()): string[] {
  const nowDate = new Date(now);
  return counts.map((_, index) => {
    const date = new Date(nowDate.getFullYear(), nowDate.getMonth() - (counts.length - 1 - index), 1);
    return `${date.getFullYear()}年${date.getMonth() + 1}月 · ${counts[index] ?? 0} ${unit}`;
  });
}

/**
 * 连续记录天数：从今天往回逐日数有记录的天数；
 * 今天还没有记录时从昨天起算（连续状态保住，不因当天还没动手清零）。
 */
export function consecutiveDays(timestamps: number[], now = Date.now()): number {
  const days = new Set(timestamps.map(timestamp => startOfDay(timestamp)));
  let cursor = startOfDay(now);
  if (!days.has(cursor)) {
    cursor -= DAY_MS;
  }
  let streak = 0;
  while (days.has(cursor)) {
    streak += 1;
    cursor -= DAY_MS;
  }
  return streak;
}

/** 瓦片里的短日期「MM-DD」，空值显示 — */
export function formatShortDate(ms: number): string {
  if (!ms) {
    return '—';
  }
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

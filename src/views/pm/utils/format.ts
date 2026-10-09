/**
 * 会话列表用的相对时间。刻意不引入日期库：
 * 规则只有四级（刚刚 / N 分钟前 / N 小时前 / 昨天 / M月d日），
 * 跨年时补年份，避免旧会话显示成含糊的「12月31日」。
 */
export function formatRelativeTime(timestamp: number, now: number = Date.now()): string {
  if (!timestamp) {
    return '';
  }

  const diff = now - timestamp;
  if (diff < 60_000) {
    return '刚刚';
  }

  const date = new Date(timestamp);
  const nowDate = new Date(now);

  const sameDay = date.toDateString() === nowDate.toDateString();
  if (!sameDay) {
    const yesterday = new Date(now);
    yesterday.setDate(nowDate.getDate() - 1);
    if (date.toDateString() !== yesterday.toDateString()) {
      const base = `${date.getMonth() + 1}月${date.getDate()}日`;
      return date.getFullYear() === nowDate.getFullYear() ? base : `${date.getFullYear()}年${base}`;
    }
    return '昨天';
  }

  const hours = Math.floor(diff / 3_600_000);
  if (hours >= 1) {
    return `${hours} 小时前`;
  }

  return `${Math.floor(diff / 60_000)} 分钟前`;
}

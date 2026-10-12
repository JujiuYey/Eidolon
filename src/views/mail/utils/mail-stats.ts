/**
 * 邮件页统计（纯函数，可单测）。
 *
 * 模板量、发送量（累计/本周/30 天分布）、失败数、最近发送时间。
 */

import type { MailTemplate, SentEmail } from '@/types/mail';
import { countSince, dailyCounts, dailyTooltips, formatShortDate, weekStartMs } from '@/utils/stats';

/** 邮件统计瓦片 + 发送趋势 */
export interface MailStats {
  templateCount: number;
  sentCount: number;
  weekSentCount: number;
  failedCount: number;
  /** 最近一次发送时间，无记录为 0 */
  lastSentAt: number;
  dailySent: number[];
  dailyTooltips: string[];
}

export function computeMailStats(templates: MailTemplate[], sentEmails: SentEmail[], now = Date.now()): MailStats {
  const sentTimestamps = sentEmails.map(email => email.sent_at);
  const daily = dailyCounts(sentTimestamps, 30, now);
  const lastSentAt = sentEmails.reduce((latest, email) => Math.max(latest, email.sent_at), 0);

  return {
    templateCount: templates.length,
    sentCount: sentEmails.length,
    weekSentCount: countSince(sentTimestamps, weekStartMs(now)),
    failedCount: sentEmails.filter(email => email.status === 'failed').length,
    lastSentAt,
    dailySent: daily,
    dailyTooltips: dailyTooltips(daily, '封发送', now),
  };
}

/** 「已发送」瓦片的提示文案：最近发送时间 */
export function lastSentHint(lastSentAt: number): string {
  return lastSentAt ? `最近 ${formatShortDate(lastSentAt)}` : '';
}

/** 按发件账户聚合发送量（降序），供「各账户发送量」条形榜 */
export function accountSendCounts(sentEmails: SentEmail[]): Array<{ label: string; value: number }> {
  const counts = new Map<string, number>();
  for (const email of sentEmails) {
    const key = email.account_email || '（未知账户）';
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return (
    [...counts.entries()]
      .map(([label, value]) => ({ label, value }))
      // 码元序而非 localeCompare：中英混排时各运行时排序不一致，测试与 UI 需要确定性
      .sort((a, b) => b.value - a.value || (a.label < b.label ? -1 : a.label > b.label ? 1 : 0))
  );
}

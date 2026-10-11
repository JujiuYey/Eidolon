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

import { describe, expect, it } from 'vitest';

import type { MailTemplate, SentEmail } from '@/types/mail';

import { computeMailStats, lastSentHint } from '../mail-stats';

const NOW = new Date(2026, 9, 14, 15).getTime(); // 周三下午
const DAY = 24 * 60 * 60 * 1000;

function makeTemplate(overrides: Partial<MailTemplate> = {}): MailTemplate {
  return {
    id: 'tpl_1',
    name: '周报通知',
    subject: 'hi {{name}}',
    body: 'hello {{name}}',
    sort: 0,
    created_at: NOW - 10 * DAY,
    updated_at: NOW - 10 * DAY,
    ...overrides,
  };
}

function makeSent(overrides: Partial<SentEmail> = {}): SentEmail {
  return {
    id: 'mail_1',
    account_id: 'acc_1',
    account_email: 'me@example.com',
    to_addresses: 'to@example.com',
    cc_addresses: '',
    subject: '周报',
    body: '内容',
    template_id: null,
    status: 'sent',
    error_message: null,
    sent_at: NOW,
    created_at: NOW,
    ...overrides,
  };
}

describe('mail stats', () => {
  it('counts templates, sends, week sends and failures', () => {
    const stats = computeMailStats(
      [makeTemplate(), makeTemplate({ id: 'tpl_2' })],
      [
        makeSent(),
        makeSent({ id: 'mail_2', sent_at: NOW - 2 * DAY, status: 'failed', error_message: 'smtp timeout' }),
        makeSent({ id: 'mail_3', sent_at: NOW - 40 * DAY }),
      ],
      NOW,
    );

    expect(stats.templateCount).toBe(2);
    expect(stats.sentCount).toBe(3);
    expect(stats.weekSentCount).toBe(2); // 今天 + 前天（周一，同属本周）
    expect(stats.failedCount).toBe(1);
    expect(stats.lastSentAt).toBe(NOW);
    expect(stats.dailySent).toHaveLength(30);
    expect(stats.dailySent[29]).toBe(1); // 今天
    expect(stats.dailySent[27]).toBe(1); // 2 天前（含失败）
    expect(stats.dailySent[0]).toBe(0); // 40 天前在窗口外
    expect(stats.dailyTooltips[29]).toContain('1 封发送');
  });

  it('handles empty history', () => {
    const stats = computeMailStats([], [], NOW);
    expect(stats.sentCount).toBe(0);
    expect(stats.weekSentCount).toBe(0);
    expect(stats.failedCount).toBe(0);
    expect(stats.lastSentAt).toBe(0);
    expect(lastSentHint(stats.lastSentAt)).toBe('');
  });

  it('lastSentHint renders the short date', () => {
    expect(lastSentHint(NOW)).toBe('最近 10-14');
  });
});

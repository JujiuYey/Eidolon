import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, defineStore, setActivePinia } from 'pinia';
import { createMailStore } from '@/stores/mail';
import type { MailService } from '@/stores/mail';
import type {
  MailTemplate,
  SendEmailRequest,
  SendEmailResult,
  SentEmail,
  SmtpAccount,
} from '@/types/mail';

function bindStore(service: MailService) {
  const useBound = defineStore('mail-test', () => createMailStore({ service }));
  return useBound();
}

function makeAccount(overrides: Partial<SmtpAccount> = {}): SmtpAccount {
  return {
    id: 'sacc_1',
    name: '公司邮箱',
    email: 'user@example.com',
    password: 'secret',
    host: 'smtp.example.com',
    port: 465,
    encryption: 'tls',
    from_name: '张三',
    enabled: true,
    sort: 0,
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

function makeTemplate(overrides: Partial<MailTemplate> = {}): MailTemplate {
  return {
    id: 'mtpl_1',
    name: '请假申请',
    subject: '{{姓名}}的{{请假类型}}申请',
    body: '尊敬的{{审批人}}：我因{{请假事由}}请假。',
    sort: 0,
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

function makeSentEmail(overrides: Partial<SentEmail> = {}): SentEmail {
  return {
    id: 'sent_1',
    account_id: 'sacc_1',
    account_email: 'user@example.com',
    to_addresses: 'boss@example.com',
    cc_addresses: '',
    subject: '张三的病假申请',
    body: '尊敬的领导：',
    template_id: 'mtpl_1',
    status: 'sent',
    error_message: null,
    sent_at: 1_000,
    created_at: 1_000,
    ...overrides,
  };
}

function makeService(overrides: Partial<MailService> = {}) {
  const accounts: SmtpAccount[] = [makeAccount()];
  const templates: MailTemplate[] = [makeTemplate()];
  const sentEmails: SentEmail[] = [];
  let nextSendResult: SendEmailResult = {
    sent_email_id: 'sent_new',
    status: 'sent',
    error_message: null,
    history_error: null,
  };
  let sendError: Error | null = null;
  const sendCalls: SendEmailRequest[] = [];

  const service: MailService = {
    listSmtpAccounts: vi.fn(async () => [...accounts]),
    upsertSmtpAccount: vi.fn(async (account: SmtpAccount) => account),
    deleteSmtpAccount: vi.fn(async (accountId: string) => accountId),
    listMailTemplates: vi.fn(async () => [...templates]),
    upsertMailTemplate: vi.fn(async (template: MailTemplate) => template),
    deleteMailTemplate: vi.fn(async (templateId: string) => {
      const index = templates.findIndex(template => template.id === templateId);
      if (index >= 0) {
        templates.splice(index, 1);
      }
      return templateId;
    }),
    sendEmail: vi.fn(async (request: SendEmailRequest) => {
      sendCalls.push(request);
      if (sendError) {
        throw sendError;
      }
      sentEmails.unshift({
        ...makeSentEmail(),
        id: nextSendResult.sent_email_id,
        subject: request.subject,
        body: request.body,
        status: nextSendResult.status,
        error_message: nextSendResult.error_message,
      });
      return nextSendResult;
    }),
    listSentEmails: vi.fn(async () => [...sentEmails]),
    deleteSentEmail: vi.fn(async (emailId: string) => emailId),
    ...overrides,
  };

  return {
    service,
    sendCalls,
    accounts,
    templates,
    sentEmails,
    setNextSendResult(value: SendEmailResult) {
      nextSendResult = value;
    },
    setSendError(error: Error) {
      sendError = error;
    },
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('createMailStore', () => {
  it('loadAll 加载三类数据并默认选中第一个启用的账户', async () => {
    const fake = makeService();
    fake.accounts.push(makeAccount({ id: 'sacc_2', name: '备用', enabled: false }));
    const store = bindStore(fake.service);

    await store.loadAll();

    expect(store.accounts).toHaveLength(2);
    expect(store.templates).toHaveLength(1);
    expect(store.selectedAccountId).toBe('sacc_1');
    expect(store.selectedAccount?.email).toBe('user@example.com');
  });

  it('selectTemplate 填充变量表，变量填写后主题正文实时渲染', () => {
    const fake = makeService();
    const store = bindStore(fake.service);

    store.templates = [makeTemplate()];
    store.selectTemplate('mtpl_1');

    expect(store.selectedTemplateId).toBe('mtpl_1');
    expect(store.composeVariables).toEqual(['姓名', '请假类型', '审批人', '请假事由']);
    expect(store.subject).toBe('{{姓名}}的{{请假类型}}申请');
    expect(store.body).toBe('尊敬的{{审批人}}：我因{{请假事由}}请假。');

    store.variableValues['姓名'] = '张三';
    store.variableValues['请假类型'] = '病假';

    expect(store.subject).toBe('张三的病假申请');
    expect(store.body).toBe('尊敬的{{审批人}}：我因{{请假事由}}请假。');
  });

  it('手动编辑主题后以手动内容为准，正文仍跟随渲染', () => {
    const fake = makeService();
    const store = bindStore(fake.service);

    store.templates = [makeTemplate()];
    store.selectTemplate('mtpl_1');

    store.setSubject('手工主题');
    store.variableValues['姓名'] = '张三';

    expect(store.subject).toBe('手工主题');
    expect(store.body).toBe('尊敬的{{审批人}}：我因{{请假事由}}请假。');
  });

  it('send 校验失败时不调用服务', async () => {
    const fake = makeService();
    const store = bindStore(fake.service);
    await store.loadAll();

    const outcome = await store.send();

    expect(outcome.ok).toBe(false);
    expect(outcome.error).toContain('收件人');
    expect(fake.sendCalls).toHaveLength(0);
  });

  it('send 成功后刷新历史并切换到已发送视图', async () => {
    const fake = makeService();
    const store = bindStore(fake.service);
    await store.loadAll();
    store.selectTemplate('mtpl_1');
    store.composeTo = 'boss@example.com';
    store.variableValues['姓名'] = '张三';
    store.variableValues['请假类型'] = '病假';
    store.variableValues['审批人'] = '李四';
    store.variableValues['请假事由'] = '感冒';

    const outcome = await store.send();

    expect(outcome.ok).toBe(true);
    expect(fake.sendCalls[0]).toMatchObject({
      account_id: 'sacc_1',
      to: 'boss@example.com',
      subject: '张三的病假申请',
      template_id: 'mtpl_1',
    });
    expect(store.viewMode).toBe('sent');
    expect(store.selectedSentEmailId).toBe('sent_new');
    expect(store.sendStatus).toBe('success');
  });

  it('send 返回 failed 状态时透出错误信息', async () => {
    const fake = makeService();
    fake.setNextSendResult({
      sent_email_id: 'sent_new',
      status: 'failed',
      error_message: '认证失败',
      history_error: null,
    });
    const store = bindStore(fake.service);
    await store.loadAll();
    store.composeTo = 'boss@example.com';
    store.setSubject('测试主题');

    const outcome = await store.send();

    expect(outcome.ok).toBe(false);
    expect(outcome.error).toBe('认证失败');
    expect(store.viewMode).toBe('compose');
    expect(store.sendStatus).toBe('error');
  });

  it('send 服务抛错时返回异常消息', async () => {
    const fake = makeService();
    fake.setSendError(new Error('连接超时'));
    const store = bindStore(fake.service);
    await store.loadAll();
    store.composeTo = 'boss@example.com';
    store.setSubject('测试主题');

    const outcome = await store.send();

    expect(outcome.ok).toBe(false);
    expect(outcome.error).toBe('连接超时');
  });

  it('resend 用历史内容预填写并回到写信视图', async () => {
    const fake = makeService();
    const store = bindStore(fake.service);
    await store.loadAll();
    store.selectTemplate('mtpl_1');

    store.resend(makeSentEmail({ subject: '旧主题', body: '旧正文', cc_addresses: 'cc@example.com' }));

    expect(store.viewMode).toBe('compose');
    expect(store.selectedTemplateId).toBe('');
    expect(store.composeTo).toBe('boss@example.com');
    expect(store.composeCc).toBe('cc@example.com');
    expect(store.subject).toBe('旧主题');
    expect(store.body).toBe('旧正文');
  });

  it('removeTemplate 在删除选中模板时重置为空白邮件', async () => {
    const fake = makeService();
    const store = bindStore(fake.service);
    await store.loadAll();
    store.selectTemplate('mtpl_1');

    await store.removeTemplate('mtpl_1');

    expect(store.selectedTemplateId).toBe('');
    expect(store.templates).toHaveLength(0);
    expect(store.subject).toBe('');
  });
});

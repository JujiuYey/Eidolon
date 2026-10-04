import type {
  MailTemplate,
  SendEmailRequest,
  SendEmailResult,
  SentEmail,
  SmtpAccount,
} from '@/types/mail';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as mailServiceApi from '@/services/mail';
import {
  buildEmptyValues,
  extractVariables,
  renderTemplate,
} from '@/views/mail/utils/template-helpers';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface MailService {
  listSmtpAccounts: () => Promise<SmtpAccount[]>;
  upsertSmtpAccount: (account: SmtpAccount) => Promise<SmtpAccount>;
  deleteSmtpAccount: (accountId: string) => Promise<string>;
  listMailTemplates: () => Promise<MailTemplate[]>;
  upsertMailTemplate: (template: MailTemplate) => Promise<MailTemplate>;
  deleteMailTemplate: (templateId: string) => Promise<string>;
  sendEmail: (request: SendEmailRequest) => Promise<SendEmailResult>;
  listSentEmails: () => Promise<SentEmail[]>;
  deleteSentEmail: (emailId: string) => Promise<string>;
}

export type MailViewMode = 'compose' | 'sent';

export type MailSendStatus = 'idle' | 'sending' | 'success' | 'error';

export interface MailSendOutcome {
  ok: boolean;
  /** 失败原因，成功时为 null */
  error: string | null;
  /** 历史写入失败提示，不影响发送本身 */
  historyError: string | null;
}

const defaultService: MailService = mailServiceApi;

/**
 * 邮件 store（工厂模式，便于测试注入 fake service）。
 *
 * 主题/正文按「模板 + 变量值」实时派生（computed），手动编辑过的字段
 * 以 override 为准；发送时取最终值，不存在渲染时序问题。
 */
export function createMailStore(options: { service?: MailService } = {}) {
  const service = options.service ?? defaultService;

  // ===== 数据 =====
  const accounts = ref<SmtpAccount[]>([]);
  const selectedAccountId = ref('');
  const templates = ref<MailTemplate[]>([]);
  const sentEmails = ref<SentEmail[]>([]);
  const selectedSentEmailId = ref('');

  // ===== 视图状态 =====
  const viewMode = ref<MailViewMode>('compose');

  // ===== 写信状态 =====
  /** 选中的模板 ID，空字符串表示空白邮件 */
  const selectedTemplateId = ref('');
  const composeTo = ref('');
  const composeCc = ref('');
  const variableValues = ref<Record<string, string>>({});
  /** 手动编辑后的主题/正文；null 表示跟随模板渲染值 */
  const subjectOverride = ref<string | null>(null);
  const bodyOverride = ref<string | null>(null);
  const sendStatus = ref<MailSendStatus>('idle');

  const selectedAccount = computed<SmtpAccount | null>(
    () => accounts.value.find(account => account.id === selectedAccountId.value) ?? null,
  );

  const selectedTemplate = computed<MailTemplate | null>(
    () => templates.value.find(template => template.id === selectedTemplateId.value) ?? null,
  );

  /** 当前模板需要的变量（空白邮件为空数组） */
  const composeVariables = computed<string[]>(() => {
    const template = selectedTemplate.value;
    if (!template) {
      return [];
    }
    return extractVariables(template.subject, template.body);
  });

  const renderedSubject = computed<string>(() => {
    const template = selectedTemplate.value;
    return template ? renderTemplate(template.subject, variableValues.value) : '';
  });

  const renderedBody = computed<string>(() => {
    const template = selectedTemplate.value;
    return template ? renderTemplate(template.body, variableValues.value) : '';
  });

  /** 发送与表单展示用的最终主题/正文 */
  const subject = computed<string>(() => subjectOverride.value ?? renderedSubject.value);
  const body = computed<string>(() => bodyOverride.value ?? renderedBody.value);

  const selectedSentEmail = computed<SentEmail | null>(
    () => sentEmails.value.find(email => email.id === selectedSentEmailId.value) ?? null,
  );

  // ===== 数据加载 =====

  async function loadAccounts(): Promise<void> {
    accounts.value = await service.listSmtpAccounts();
    if (!accounts.value.some(account => account.id === selectedAccountId.value)) {
      selectedAccountId.value = accounts.value.find(account => account.enabled)?.id
        ?? accounts.value[0]?.id
        ?? '';
    }
  }

  async function loadTemplates(): Promise<void> {
    templates.value = await service.listMailTemplates();
    if (!templates.value.some(template => template.id === selectedTemplateId.value)) {
      selectedTemplateId.value = '';
    }
  }

  async function loadSentEmails(): Promise<void> {
    sentEmails.value = await service.listSentEmails();
    if (!sentEmails.value.some(email => email.id === selectedSentEmailId.value)) {
      selectedSentEmailId.value = sentEmails.value[0]?.id ?? '';
    }
  }

  async function loadAll(): Promise<void> {
    await Promise.all([loadAccounts(), loadTemplates(), loadSentEmails()]);
  }

  // ===== 写信 =====

  /**
   * 选择模板并重置写信表单。templateId 传空字符串表示空白邮件。
   */
  function selectTemplate(templateId: string): void {
    selectedTemplateId.value = templateId;
    const template = templates.value.find(item => item.id === templateId) ?? null;
    const variables = template ? extractVariables(template.subject, template.body) : [];

    variableValues.value = buildEmptyValues(variables);
    subjectOverride.value = null;
    bodyOverride.value = null;
  }

  function setSubject(value: string): void {
    subjectOverride.value = value;
  }

  function setBody(value: string): void {
    bodyOverride.value = value;
  }

  /**
   * 从发送历史预填写写信表单（重新发送）。
   */
  function resend(sentEmail: SentEmail): void {
    selectTemplate('');
    viewMode.value = 'compose';
    composeTo.value = sentEmail.to_addresses;
    composeCc.value = sentEmail.cc_addresses;
    subjectOverride.value = sentEmail.subject;
    bodyOverride.value = sentEmail.body;
  }

  function validateCompose(): string | null {
    if (!selectedAccountId.value) {
      return '请先在 设置 → 邮件账户 中配置发件账户';
    }
    if (!selectedAccount.value?.enabled) {
      return '所选发件账户已被停用';
    }
    if (!composeTo.value.trim()) {
      return '请填写收件人';
    }
    const missing = composeVariables.value.filter(
      name => (variableValues.value[name] ?? '').trim() === '',
    );
    if (missing.length > 0) {
      return `请填写模板变量：${missing.join('、')}`;
    }
    if (!subject.value.trim()) {
      return '请填写邮件主题';
    }
    return null;
  }

  /**
   * 发送邮件。无论发送成功与否（只要发起过真实发送）都会刷新发送历史；
   * 发送成功后自动切换到「已发送」视图并选中新记录。
   */
  async function send(): Promise<MailSendOutcome> {
    const validationError = validateCompose();
    if (validationError) {
      return { ok: false, error: validationError, historyError: null };
    }

    sendStatus.value = 'sending';
    const request: SendEmailRequest = {
      account_id: selectedAccountId.value,
      to: composeTo.value,
      cc: composeCc.value,
      subject: subject.value,
      body: body.value,
      template_id: selectedTemplateId.value,
    };

    try {
      const result = await service.sendEmail(request);
      await loadSentEmails();

      if (result.status === 'failed') {
        sendStatus.value = 'error';
        return { ok: false, error: result.error_message ?? '邮件发送失败', historyError: null };
      }

      sendStatus.value = 'success';
      if (result.sent_email_id) {
        selectedSentEmailId.value = result.sent_email_id;
      }
      viewMode.value = 'sent';
      return { ok: true, error: null, historyError: result.history_error };
    } catch (error) {
      sendStatus.value = 'error';
      return {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
        historyError: null,
      };
    } finally {
      if (sendStatus.value === 'sending') {
        sendStatus.value = 'idle';
      }
    }
  }

  // ===== 模板管理（写信页内的轻量维护）=====

  async function saveTemplate(template: MailTemplate): Promise<MailTemplate> {
    const saved = await service.upsertMailTemplate(template);
    await loadTemplates();
    return saved;
  }

  async function removeTemplate(templateId: string): Promise<void> {
    await service.deleteMailTemplate(templateId);
    if (selectedTemplateId.value === templateId) {
      selectTemplate('');
    }
    await loadTemplates();
  }

  async function removeSentEmail(emailId: string): Promise<void> {
    await service.deleteSentEmail(emailId);
    await loadSentEmails();
  }

  return {
    // 数据
    accounts,
    selectedAccountId,
    templates,
    sentEmails,
    selectedSentEmailId,
    selectedAccount,
    selectedTemplate,
    selectedSentEmail,
    // 视图
    viewMode,
    // 写信
    selectedTemplateId,
    composeTo,
    composeCc,
    variableValues,
    composeVariables,
    renderedSubject,
    renderedBody,
    subject,
    body,
    sendStatus,
    // 动作
    loadAll,
    loadAccounts,
    loadTemplates,
    loadSentEmails,
    selectTemplate,
    setSubject,
    setBody,
    resend,
    send,
    saveTemplate,
    removeTemplate,
    removeSentEmail,
  };
}

export type MailStore = ReturnType<typeof createMailStore>;

export const useMailStore = defineStore('mail', () => createMailStore());

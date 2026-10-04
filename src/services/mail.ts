import type {
  MailTemplate,
  SendEmailRequest,
  SendEmailResult,
  SentEmail,
  SmtpAccount,
  TestSmtpConnectionRequest,
  TestSmtpConnectionResponse,
} from '@/types/mail';
import { invoke } from '@tauri-apps/api/core';

export async function listSmtpAccounts(): Promise<SmtpAccount[]> {
  return invoke<SmtpAccount[]>('list_smtp_accounts');
}

export async function upsertSmtpAccount(account: SmtpAccount): Promise<SmtpAccount> {
  return invoke<SmtpAccount>('upsert_smtp_account', { account });
}

export async function deleteSmtpAccount(accountId: string): Promise<string> {
  return invoke<string>('delete_smtp_account', { accountId });
}

export async function listMailTemplates(): Promise<MailTemplate[]> {
  return invoke<MailTemplate[]>('list_mail_templates');
}

export async function upsertMailTemplate(template: MailTemplate): Promise<MailTemplate> {
  return invoke<MailTemplate>('upsert_mail_template', { template });
}

export async function deleteMailTemplate(templateId: string): Promise<string> {
  return invoke<string>('delete_mail_template', { templateId });
}

export async function sendEmail(request: SendEmailRequest): Promise<SendEmailResult> {
  return invoke<SendEmailResult>('send_email', { request });
}

export async function testSmtpConnection(
  request: TestSmtpConnectionRequest,
): Promise<TestSmtpConnectionResponse> {
  return invoke<TestSmtpConnectionResponse>('test_smtp_connection', { request });
}

export async function listSentEmails(): Promise<SentEmail[]> {
  return invoke<SentEmail[]>('list_sent_emails');
}

export async function deleteSentEmail(emailId: string): Promise<string> {
  return invoke<string>('delete_sent_email', { emailId });
}

export async function clearSentEmails(): Promise<number> {
  return invoke<number>('clear_sent_emails');
}

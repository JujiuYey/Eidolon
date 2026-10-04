/**
 * 邮件功能的 Tauri DTO 类型。
 *
 * 与后端 `src-tauri/src/models/email.rs` 的 serde 结构一一对应，
 * 字段保持 snake_case 直映（同 `types/provider.ts` 约定）。
 */

export type SmtpEncryption = 'none' | 'starttls' | 'tls';

export type EmailSendStatus = 'sent' | 'failed';

export interface SmtpAccount {
  id: string;
  /** 账户显示名（如 "公司邮箱"） */
  name: string;
  /** 发件邮箱地址，同时作为 SMTP 登录用户名 */
  email: string;
  /** SMTP 密码/授权码 */
  password: string;
  host: string;
  port: number;
  encryption: SmtpEncryption;
  /** 发件人显示名，可为空 */
  from_name: string;
  enabled: boolean;
  sort: number;
  created_at: number;
  updated_at: number;
}

export interface MailTemplate {
  id: string;
  name: string;
  /** 主题模板，含 {{变量}} 占位符 */
  subject: string;
  /** 正文模板，含 {{变量}} 占位符 */
  body: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

export interface SentEmail {
  id: string;
  /** 所用账户；账户删除后为 null */
  account_id: string | null;
  /** 发件邮箱快照 */
  account_email: string;
  to_addresses: string;
  cc_addresses: string;
  /** 渲染后的最终主题 */
  subject: string;
  /** 渲染后的最终正文 */
  body: string;
  /** 使用的模板；模板删除后为 null */
  template_id: string | null;
  status: EmailSendStatus;
  error_message: string | null;
  sent_at: number;
  created_at: number;
}

export interface SendEmailRequest {
  account_id: string;
  /** 收件人列表，逗号/分号分隔 */
  to: string;
  /** 抄送列表，逗号/分号分隔，可为空 */
  cc: string;
  /** 渲染后的最终主题 */
  subject: string;
  /** 渲染后的最终正文 */
  body: string;
  /** 使用的模板 ID，空白邮件传空字符串 */
  template_id: string;
}

export interface SendEmailResult {
  sent_email_id: string;
  status: EmailSendStatus;
  error_message: string | null;
  /** 历史写入失败时提示，不影响发送结果 */
  history_error: string | null;
}

export interface TestSmtpConnectionRequest {
  email: string;
  password: string;
  host: string;
  port: number;
  encryption: SmtpEncryption;
  from_name: string;
}

export interface TestSmtpConnectionResponse {
  success: boolean;
  message: string;
}

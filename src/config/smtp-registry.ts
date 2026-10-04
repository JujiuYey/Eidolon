import type { SmtpEncryption } from '@/types/mail';

/**
 * SMTP 服务商预设，用于账户表单快速填充。
 * 自定义 preset 的 host 为空，要求用户手动输入。
 */
export interface SmtpRegistryItem {
  preset_id: string;
  name: string;
  host: string;
  port: number;
  encryption: SmtpEncryption;
  /** 凭据填写提示，展示在密码输入框下方 */
  hint?: string;
}

export const SMTP_REGISTRY: SmtpRegistryItem[] = [
  {
    preset_id: 'aliyun-qiye',
    name: '阿里云企业邮箱',
    host: 'smtp.qiye.aliyun.com',
    port: 465,
    encryption: 'tls',
    hint: '密码为邮箱登录密码，若不可用请在邮箱网页端开启 IMAP/SMTP 服务',
  },
  {
    preset_id: 'aliyun',
    name: '阿里云邮箱（个人）',
    host: 'smtp.aliyun.com',
    port: 465,
    encryption: 'tls',
  },
  {
    preset_id: 'qq',
    name: 'QQ 邮箱',
    host: 'smtp.qq.com',
    port: 465,
    encryption: 'tls',
    hint: '需在 QQ 邮箱设置中开启 SMTP 服务并使用授权码登录',
  },
  {
    preset_id: '163',
    name: '网易 163 邮箱',
    host: 'smtp.163.com',
    port: 465,
    encryption: 'tls',
    hint: '需在邮箱设置中开启 SMTP 服务并使用授权码登录',
  },
  {
    preset_id: 'gmail',
    name: 'Gmail',
    host: 'smtp.gmail.com',
    port: 465,
    encryption: 'tls',
    hint: '需开启两步验证并使用应用专用密码',
  },
  {
    preset_id: 'outlook',
    name: 'Outlook / Microsoft 365',
    host: 'smtp.office365.com',
    port: 587,
    encryption: 'starttls',
  },
  {
    preset_id: 'custom',
    name: '自定义',
    host: '',
    port: 465,
    encryption: 'tls',
  },
];

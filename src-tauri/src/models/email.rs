use serde::{Deserialize, Serialize};

/// 发送历史保留条数（全局）
pub const MAX_SENT_EMAIL_HISTORY: usize = 500;

/// 邮件正文保存上限：1 MiB
pub const MAX_EMAIL_BODY_BYTES: usize = 1024 * 1024;

/// SMTP 加密方式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpEncryption {
    /// 明文，不加密（仅限内网中转等场景）
    None,
    /// STARTTLS（通常 587 端口）
    Starttls,
    /// 直连 TLS（通常 465 端口）
    #[default]
    Tls,
}

impl SmtpEncryption {
    pub fn as_str(self) -> &'static str {
        match self {
            SmtpEncryption::None => "none",
            SmtpEncryption::Starttls => "starttls",
            SmtpEncryption::Tls => "tls",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "none" => Ok(SmtpEncryption::None),
            "starttls" => Ok(SmtpEncryption::Starttls),
            "tls" => Ok(SmtpEncryption::Tls),
            other => Err(format!("不支持的加密方式: {other}")),
        }
    }
}

/// 发送状态。失败也会落一条历史，便于排查
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailSendStatus {
    #[default]
    Sent,
    Failed,
}

impl EmailSendStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            EmailSendStatus::Sent => "sent",
            EmailSendStatus::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "sent" => Ok(EmailSendStatus::Sent),
            "failed" => Ok(EmailSendStatus::Failed),
            other => Err(format!("不支持的发送状态: {other}")),
        }
    }
}

/// SMTP 发信账户。`email` 同时作为登录用户名与默认发件地址
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmtpAccount {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub email: String,

    #[serde(default)]
    pub password: String,

    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub port: i64,

    #[serde(default)]
    pub encryption: SmtpEncryption,

    #[serde(default)]
    pub from_name: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

impl Default for SmtpAccount {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            email: String::new(),
            password: String::new(),
            host: String::new(),
            port: 465,
            encryption: SmtpEncryption::Tls,
            from_name: String::new(),
            enabled: true,
            sort: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// 邮件模板。主题与正文含 `{{变量}}` 占位符，渲染在前端完成
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailTemplate {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub subject: String,

    #[serde(default)]
    pub body: String,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// 发送历史。保存的是渲染后的最终文本
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentEmail {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub account_id: Option<String>,

    #[serde(default)]
    pub account_email: String,

    #[serde(default)]
    pub to_addresses: String,

    #[serde(default)]
    pub cc_addresses: String,

    #[serde(default)]
    pub subject: String,

    #[serde(default)]
    pub body: String,

    #[serde(default)]
    pub template_id: Option<String>,

    #[serde(default)]
    pub status: EmailSendStatus,

    #[serde(default)]
    pub error_message: Option<String>,

    #[serde(default)]
    pub sent_at: i64,

    #[serde(default)]
    pub created_at: i64,
}

/// 发送请求。主题/正文必须是渲染后的最终文本，后端不做变量替换
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendEmailRequest {
    #[serde(default)]
    pub account_id: String,

    /// 收件人列表，逗号/分号分隔
    #[serde(default)]
    pub to: String,

    /// 抄送列表，逗号/分号分隔，可为空
    #[serde(default)]
    pub cc: String,

    #[serde(default)]
    pub subject: String,

    #[serde(default)]
    pub body: String,

    /// 使用的模板 ID，可为空（空白邮件）
    #[serde(default)]
    pub template_id: String,
}

/// 发送结果。发送失败也返回 `Ok`（status = failed），由前端决定提示方式
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SendEmailResult {
    #[serde(default)]
    pub sent_email_id: String,

    pub status: EmailSendStatus,

    #[serde(default)]
    pub error_message: Option<String>,

    /// 历史写入失败时提示，不影响发送结果
    #[serde(default)]
    pub history_error: Option<String>,
}

/// 校验端口取值范围
pub fn validate_port(port: i64) -> Result<i64, String> {
    if !(1..=65535).contains(&port) {
        return Err("端口必须在 1-65535 之间".to_string());
    }
    Ok(port)
}

/// 校验邮箱地址格式（仅做基本检查，真实可达性由 SMTP 握手验证）
pub fn validate_email(email: &str, field: &str) -> Result<String, String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    let at = trimmed
        .find('@')
        .ok_or_else(|| format!("{field}格式不正确: {trimmed}"))?;
    if at == 0 || at == trimmed.len() - 1 || trimmed[at + 1..].contains('@') {
        return Err(format!("{field}格式不正确: {trimmed}"));
    }
    Ok(trimmed.to_string())
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_encryption_round_trips_through_string() {
        for encryption in [
            SmtpEncryption::None,
            SmtpEncryption::Starttls,
            SmtpEncryption::Tls,
        ] {
            assert_eq!(
                SmtpEncryption::parse(encryption.as_str()).expect("parse"),
                encryption
            );
        }
        assert!(SmtpEncryption::parse("ssl").is_err());
    }

    #[test]
    fn email_send_status_round_trips_through_string() {
        for status in [EmailSendStatus::Sent, EmailSendStatus::Failed] {
            assert_eq!(EmailSendStatus::parse(status.as_str()).expect("parse"), status);
        }
        assert!(EmailSendStatus::parse("pending").is_err());
    }

    #[test]
    fn smtp_encryption_defaults_to_tls() {
        assert_eq!(SmtpAccount::default().encryption, SmtpEncryption::Tls);
        assert_eq!(SmtpAccount::default().enabled, true);
    }

    #[test]
    fn validate_port_accepts_valid_ports_only() {
        assert_eq!(validate_port(465).expect("465"), 465);
        assert_eq!(validate_port(1).expect("1"), 1);
        assert_eq!(validate_port(65535).expect("65535"), 65535);
        assert!(validate_port(0).is_err());
        assert!(validate_port(65536).is_err());
        assert!(validate_port(-1).is_err());
    }

    #[test]
    fn validate_email_checks_basic_shape() {
        assert_eq!(
            validate_email(" user@example.com ", "邮箱").expect("valid"),
            "user@example.com"
        );
        assert!(validate_email("", "邮箱").is_err());
        assert!(validate_email("not-an-email", "邮箱").is_err());
        assert!(validate_email("@example.com", "邮箱").is_err());
        assert!(validate_email("user@", "邮箱").is_err());
        assert!(validate_email("a@b@c.com", "邮箱").is_err());
    }
}

//! 邮件发送编排：构建 SMTP 连接、发送渲染后的邮件、落发送历史。
//!
//! 模板渲染在前端完成，本模块只接触渲染后的最终文本。发送失败同样
//! 落一条失败历史（`status = failed`），历史写入失败透出
//! `SendEmailResult::history_error`，不吞掉发送结果本身。

use lettre::{
    message::Mailbox,
    transport::smtp::authentication::Credentials,
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::sent_email_repo::SentEmailRepository;
use crate::db::repositories::smtp_account_repo::SmtpAccountRepository;
use crate::models::email::{
    validate_email, EmailSendStatus, SendEmailRequest, SendEmailResult, SentEmail, SmtpAccount,
    SmtpEncryption,
};

/// 发送一封邮件并记录历史。
///
/// 账户缺失/停用、收件人不合法等调用错误返回 `Err`；
/// SMTP 发送失败返回 `Ok`（status = failed），由前端决定提示方式。
pub async fn send_email(
    database: &ApiClientDatabase,
    request: &SendEmailRequest,
) -> Result<SendEmailResult, String> {
    let account_id = request.account_id.trim();
    if account_id.is_empty() {
        return Err("请先选择发件账户".to_string());
    }

    let account = SmtpAccountRepository::new(database)
        .get(account_id)?
        .ok_or_else(|| format!("未找到 id 为 {account_id} 的邮件账户"))?;

    if !account.enabled {
        return Err("邮件账户已被停用，请先在设置中启用".to_string());
    }

    let to = parse_mailboxes(&request.to, "收件人")?;
    let cc = if request.cc.trim().is_empty() {
        Vec::new()
    } else {
        parse_mailboxes(&request.cc, "抄送")?
    };
    let subject = validate_subject(&request.subject)?;

    let template_id = request.template_id.trim();
    let template_id = (!template_id.is_empty()).then(|| template_id.to_string());

    let transport = build_transport(&account)?;
    let message = build_message(&account, &to, &cc, &subject, &request.body)?;

    let send_outcome = transport.send(message).await;

    let (status, error_message) = match &send_outcome {
        Ok(_) => (EmailSendStatus::Sent, None),
        Err(error) => (
            EmailSendStatus::Failed,
            Some(format!("邮件发送失败: {error}")),
        ),
    };

    let to_addresses = mailboxes_to_string(&to);
    let cc_addresses = mailboxes_to_string(&cc);

    let record = SentEmail {
        account_id: Some(account.id.clone()),
        account_email: account.email.clone(),
        to_addresses,
        cc_addresses,
        subject,
        body: request.body.clone(),
        template_id,
        status,
        error_message: error_message.clone(),
        ..Default::default()
    };

    let mut result = SendEmailResult {
        status,
        error_message,
        ..Default::default()
    };

    match SentEmailRepository::new(database).append(&record) {
        Ok(saved) => result.sent_email_id = saved.id,
        Err(error) => result.history_error = Some(format!("发送记录未保存: {error}")),
    }

    Ok(result)
}

/// 测试 SMTP 连通性：给发件地址自己发一封测试邮件。
/// SMTP 认证在发送时才发生，因此这一步能全链路验证服务器、端口与凭据。
pub async fn test_connection(account: &SmtpAccount) -> Result<(), String> {
    let transport = build_transport(account)?;
    let to = parse_mailboxes(&account.email, "邮箱地址")?;
    let message = build_message(
        account,
        &to,
        &[],
        "Eidolon 邮件账户测试",
        "这是一封来自 Eidolon 的测试邮件。收到即表示该 SMTP 账户配置正确。",
    )?;

    transport
        .send(message)
        .await
        .map_err(|error| format!("测试邮件发送失败: {error}"))?;

    Ok(())
}

/// 按账户的加密方式构建带凭据的异步 SMTP transport
fn build_transport(account: &SmtpAccount) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let host = account.host.trim();
    if host.is_empty() {
        return Err("SMTP 服务器地址不能为空".to_string());
    }

    let builder = match account.encryption {
        SmtpEncryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host),
        SmtpEncryption::Starttls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host),
        SmtpEncryption::None => Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)),
    }
    .map_err(|error| format!("构建 SMTP 客户端失败: {error}"))?;

    Ok(builder
        .credentials(Credentials::new(
            account.email.trim().to_string(),
            account.password.clone(),
        ))
        .build())
}

/// 构建纯文本邮件。主题与收件人已在上层校验
fn build_message(
    account: &SmtpAccount,
    to: &[Mailbox],
    cc: &[Mailbox],
    subject: &str,
    body: &str,
) -> Result<Message, String> {
    let mut builder = Message::builder().from(sender_mailbox(account)?);

    for mailbox in to {
        builder = builder.to(mailbox.clone());
    }
    for mailbox in cc {
        builder = builder.cc(mailbox.clone());
    }

    builder
        .subject(subject.to_string())
        .body(body.to_string())
        .map_err(|error| format!("构建邮件失败: {error}"))
}

/// 发件人 Mailbox：`from_name` 为空时只保留邮箱地址
fn sender_mailbox(account: &SmtpAccount) -> Result<Mailbox, String> {
    let email = validate_email(&account.email, "发件邮箱地址")?;
    let address = email
        .parse::<Address>()
        .map_err(|error| format!("发件邮箱地址格式不正确: {error}"))?;

    let name = account.from_name.trim();
    if name.is_empty() {
        Ok(Mailbox::new(None, address))
    } else {
        Ok(Mailbox::new(Some(name.to_string()), address))
    }
}

/// 解析逗号/分号分隔的地址列表，同时兼容中英文标点
fn parse_mailboxes(raw: &str, field: &str) -> Result<Vec<Mailbox>, String> {
    let mut mailboxes = Vec::new();

    for part in raw.split([',', ';', '，', '；', '\n']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let mailbox = part
            .parse::<Mailbox>()
            .map_err(|error| format!("{field}中的地址不合法: {part} ({error})"))?;
        mailboxes.push(mailbox);
    }

    if mailboxes.is_empty() {
        return Err(format!("{field}不能为空"));
    }

    Ok(mailboxes)
}

fn validate_subject(subject: &str) -> Result<String, String> {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return Err("邮件主题不能为空".to_string());
    }
    if trimmed.contains('\r') || trimmed.contains('\n') {
        return Err("邮件主题不能包含换行符".to_string());
    }
    Ok(trimmed.to_string())
}

fn mailboxes_to_string(mailboxes: &[Mailbox]) -> String {
    mailboxes
        .iter()
        .map(|mailbox| mailbox.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

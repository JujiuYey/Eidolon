use serde::{Deserialize, Serialize};

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::mail_template_repo::MailTemplateRepository;
use crate::db::repositories::sent_email_repo::SentEmailRepository;
use crate::db::repositories::smtp_account_repo::SmtpAccountRepository;
use crate::models::email::{
    MailTemplate, SendEmailRequest, SendEmailResult, SentEmail, SmtpAccount, SmtpEncryption,
};
use crate::services::email;

// ===== 邮件账户 =====

#[tauri::command]
pub fn list_smtp_accounts(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<SmtpAccount>, String> {
    SmtpAccountRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_smtp_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account: SmtpAccount,
) -> Result<SmtpAccount, String> {
    SmtpAccountRepository::new(&database).upsert(&account)
}

#[tauri::command]
pub fn delete_smtp_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account_id: String,
) -> Result<String, String> {
    SmtpAccountRepository::new(&database).delete(&account_id)
}

// ===== 邮件模板 =====

#[tauri::command]
pub fn list_mail_templates(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<MailTemplate>, String> {
    MailTemplateRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_mail_template(
    database: tauri::State<'_, ApiClientDatabase>,
    template: MailTemplate,
) -> Result<MailTemplate, String> {
    MailTemplateRepository::new(&database).upsert(&template)
}

#[tauri::command]
pub fn delete_mail_template(
    database: tauri::State<'_, ApiClientDatabase>,
    template_id: String,
) -> Result<String, String> {
    MailTemplateRepository::new(&database).delete(&template_id)
}

// ===== 发送 =====

#[tauri::command]
pub async fn send_email(
    database: tauri::State<'_, ApiClientDatabase>,
    request: SendEmailRequest,
) -> Result<SendEmailResult, String> {
    email::send_email(database.inner(), &request).await
}

#[derive(Debug, Deserialize)]
pub struct TestSmtpConnectionRequest {
    pub email: String,
    pub password: String,
    pub host: String,
    pub port: i64,
    pub encryption: SmtpEncryption,
    pub from_name: String,
}

#[derive(Debug, Serialize)]
pub struct TestSmtpConnectionResponse {
    pub success: bool,
    pub message: String,
}

/// 发送测试邮件验证 SMTP 配置，收件人是配置的发件邮箱本身
#[tauri::command]
pub async fn test_smtp_connection(
    request: TestSmtpConnectionRequest,
) -> Result<TestSmtpConnectionResponse, String> {
    let account = SmtpAccount {
        email: request.email,
        password: request.password,
        host: request.host,
        port: request.port,
        encryption: request.encryption,
        from_name: request.from_name,
        ..Default::default()
    };

    email::test_connection(&account).await?;

    Ok(TestSmtpConnectionResponse {
        success: true,
        message: "测试邮件已发送，请查收".to_string(),
    })
}

// ===== 发送历史 =====

#[tauri::command]
pub fn list_sent_emails(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<SentEmail>, String> {
    SentEmailRepository::new(&database).list()
}

#[tauri::command]
pub fn delete_sent_email(
    database: tauri::State<'_, ApiClientDatabase>,
    email_id: String,
) -> Result<String, String> {
    SentEmailRepository::new(&database).delete(&email_id)
}

#[tauri::command]
pub fn clear_sent_emails(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<usize, String> {
    SentEmailRepository::new(&database).clear()
}

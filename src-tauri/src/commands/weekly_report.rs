use crate::db::api_client::ApiClientDatabase;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::weekly_report_repo::{
    WeeklyReportHistoryRepository, WeeklyReportSourceRepository,
};
use crate::models::weekly_report::{
    FetchCommitsRequest, PolishWeeklyReportInput, PolishWeeklyReportResult, RepoCommitLog,
    WeeklyReportEntry, WeeklyReportRepo,
};
use crate::services::weekly_report;

// ===== 周报仓库 =====

#[tauri::command]
pub fn list_weekly_report_repos(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<WeeklyReportRepo>, String> {
    WeeklyReportSourceRepository::new(&database).list()
}

/// 添加仓库。先校验目录存在且是 git 仓库，再落库（路径唯一）
#[tauri::command]
pub async fn add_weekly_report_repo(
    database: tauri::State<'_, ApiClientDatabase>,
    path: String,
) -> Result<WeeklyReportRepo, String> {
    let validated = weekly_report::validate_git_repo(&path).await?;

    let name = validated
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| validated.to_string_lossy().to_string());

    WeeklyReportSourceRepository::new(&database).insert(&validated.to_string_lossy(), &name)
}

#[tauri::command]
pub fn remove_weekly_report_repo(
    database: tauri::State<'_, ApiClientDatabase>,
    repo_id: String,
) -> Result<String, String> {
    WeeklyReportSourceRepository::new(&database).delete(&repo_id)
}

/// 拉取各仓库在区间内的提交。单仓库失败不中断，错误随该仓库结果返回
#[tauri::command]
pub async fn fetch_weekly_report_commits(
    database: tauri::State<'_, ApiClientDatabase>,
    request: FetchCommitsRequest,
) -> Result<Vec<RepoCommitLog>, String> {
    weekly_report::fetch_commits(&database, &request).await
}

// ===== 周报历史 =====

#[tauri::command]
pub fn list_weekly_reports(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<WeeklyReportEntry>, String> {
    WeeklyReportHistoryRepository::new(&database).list()
}

#[tauri::command]
pub fn save_weekly_report(
    database: tauri::State<'_, ApiClientDatabase>,
    report: WeeklyReportEntry,
) -> Result<WeeklyReportEntry, String> {
    WeeklyReportHistoryRepository::new(&database).upsert(&report)
}

#[tauri::command]
pub fn delete_weekly_report(
    database: tauri::State<'_, ApiClientDatabase>,
    report_id: String,
) -> Result<String, String> {
    WeeklyReportHistoryRepository::new(&database).delete(&report_id)
}

/// 把周报正文写入本地文件（配合前端系统保存对话框）
#[tauri::command]
pub fn export_weekly_report(path: String, content: String) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("导出路径不能为空".to_string());
    }

    std::fs::write(trimmed, content.as_bytes())
        .map_err(|error| format!("写入文件失败: {error}"))?;

    Ok(trimmed.to_string())
}

// ===== AI 润色 =====

/// 用默认模型润色周报草稿，返回润色后的 Markdown 与所用模型标识
#[tauri::command]
pub async fn polish_weekly_report(
    store: tauri::State<'_, LocalJsonStore>,
    input: PolishWeeklyReportInput,
) -> Result<PolishWeeklyReportResult, String> {
    weekly_report::polish_report(&store, &input).await
}

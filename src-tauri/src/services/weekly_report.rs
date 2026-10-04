//! 周报服务：本地 git 提交拉取与 AI 润色。
//!
//! git 通过命令行调用（与项目"不引入 git2"的现状一致），每个仓库独立执行，
//! 单仓库失败不影响其他仓库。所有错误以中文 `String` 返回。

use std::path::Path;
use std::time::Duration;

use futures_util::future::join_all;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::openai::Client;
use tokio::time::timeout;

use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::db::repositories::weekly_report_repo::WeeklyReportSourceRepository;
use crate::db::api_client::ApiClientDatabase;
use crate::models::weekly_report::{
    FetchCommitsRequest, PolishWeeklyReportInput, PolishWeeklyReportResult, RepoCommitLog,
    WeeklyReportAuthorMode, WeeklyReportCommit, WeeklyReportRepo,
    WEEKLY_REPORT_POLISH_MODEL_KEY,
};

/// 单个仓库 git log 的超时时间
const GIT_LOG_TIMEOUT: Duration = Duration::from_secs(30);

/// 提交字段分隔符（ASCII Unit Separator），避免与提交主题中的普通字符冲突
const FIELD_SEPARATOR: char = '\u{1f}';

const POLISH_SYSTEM_PROMPT: &str = "你是周报润色助手。用户会提供一份基于 git 提交记录整理的周报草稿。\
请把它润色成通顺、专业、简洁的中文周报：保留 Markdown 结构与全部事实内容，合并重复项，\
规范措辞，但不要编造草稿中不存在的工作，也不要添加夸张的评价。只输出润色后的周报正文。";

// ===== 提交拉取 =====

/// 校验目录是一个可用的 git 仓库，返回归一化后的路径
pub async fn validate_git_repo(raw_path: &str) -> Result<std::path::PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("仓库路径不能为空".to_string());
    }

    let path = std::path::PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(format!("仓库路径必须是绝对路径: {trimmed}"));
    }
    if !path.is_dir() {
        return Err(format!("目录不存在: {}", path.display()));
    }

    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&path)
        .output()
        .await
        .map_err(|error| format!("启动 git 失败（请确认已安装 git）: {error}"))?;

    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != "true" {
        return Err(format!("该目录不是 git 仓库: {}", path.display()));
    }

    Ok(path)
}

/// 读取仓库 `git config user.email`，未配置时返回 `None`
async fn repo_author_email(path: &Path) -> Result<Option<String>, String> {
    let output = tokio::process::Command::new("git")
        .args(["config", "user.email"])
        .current_dir(path)
        .output()
        .await
        .map_err(|error| format!("启动 git 失败: {error}"))?;

    let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if email.is_empty() {
        return Ok(None);
    }
    Ok(Some(email))
}

/// 按请求拉取各仓库在区间内的提交。`repo_ids` 为空时取全部仓库
pub async fn fetch_commits(
    database: &ApiClientDatabase,
    request: &FetchCommitsRequest,
) -> Result<Vec<RepoCommitLog>, String> {
    if request.until_ms < request.since_ms {
        return Err("统计区间无效：结束时间早于开始时间".to_string());
    }

    let repos = {
        let repository = WeeklyReportSourceRepository::new(database);
        if request.repo_ids.is_empty() {
            repository.list()?
        } else {
            repository.list_by_ids(&request.repo_ids)?
        }
    };

    if repos.is_empty() {
        return Err("请先添加要统计的代码仓库".to_string());
    }

    let since = ms_to_rfc3339(request.since_ms)?;
    let until = ms_to_rfc3339(request.until_ms)?;
    let since = since.as_str();
    let until = until.as_str();

    let tasks = repos.iter().map(|repo| async move {
        let author = resolve_author(repo, request).await;
        match author {
            Ok(author) => {
                let outcome = run_git_log(Path::new(&repo.path), &since, &until, author.as_deref())
                    .await;
                match outcome {
                    Ok(commits) => RepoCommitLog {
                        repo_id: repo.id.clone(),
                        repo_name: repo.name.clone(),
                        commits,
                        error: None,
                    },
                    Err(error) => RepoCommitLog {
                        repo_id: repo.id.clone(),
                        repo_name: repo.name.clone(),
                        commits: Vec::new(),
                        error: Some(error),
                    },
                }
            }
            Err(error) => RepoCommitLog {
                repo_id: repo.id.clone(),
                repo_name: repo.name.clone(),
                commits: Vec::new(),
                error: Some(error),
            },
        }
    });

    Ok(join_all(tasks).await)
}

/// 解析该仓库实际生效的 `--author` 值；`Ok(None)` 表示不过滤
async fn resolve_author(
    repo: &WeeklyReportRepo,
    request: &FetchCommitsRequest,
) -> Result<Option<String>, String> {
    match request.author_mode {
        WeeklyReportAuthorMode::All => Ok(None),
        WeeklyReportAuthorMode::Custom => {
            let author = request.author.trim();
            if author.is_empty() {
                return Err("作者过滤方式为自定义，但未填写作者".to_string());
            }
            Ok(Some(author.to_string()))
        }
        WeeklyReportAuthorMode::Auto => {
            let email = repo_author_email(Path::new(&repo.path)).await?;
            email.map(Some).ok_or_else(|| {
                format!(
                    "仓库 {} 未配置 user.email，无法按作者过滤；请在仓库中执行 git config 后重试，或改用其他过滤方式",
                    repo.name
                )
            })
        }
    }
}

async fn run_git_log(
    path: &Path,
    since: &str,
    until: &str,
    author: Option<&str>,
) -> Result<Vec<WeeklyReportCommit>, String> {
    let mut command = tokio::process::Command::new("git");
    command
        .current_dir(path)
        .args([
            "log",
            "--all",
            "--no-merges",
            &format!("--since={since}"),
            &format!("--until={until}"),
            &format!(
                "--pretty=format:%H{FIELD_SEPARATOR}%h{FIELD_SEPARATOR}%s{FIELD_SEPARATOR}%an{FIELD_SEPARATOR}%aI"
            ),
        ]);
    if let Some(author) = author {
        command.arg(format!("--author={author}"));
    }

    let output = timeout(GIT_LOG_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("读取提交记录超时（超过 {} 秒）", GIT_LOG_TIMEOUT.as_secs()))?
        .map_err(|error| format!("执行 git log 失败: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log 失败: {}", stderr.trim()));
    }

    Ok(parse_log_output(&String::from_utf8_lossy(&output.stdout)))
}

/// 解析 `git log --pretty=format:%H%x1f%h%x1f%s%x1f%an%x1f%aI` 的输出
fn parse_log_output(raw: &str) -> Vec<WeeklyReportCommit> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split(FIELD_SEPARATOR);
            let hash = fields.next()?.trim().to_string();
            let short_hash = fields.next()?.trim().to_string();
            let subject = fields.next()?.trim().to_string();
            let author_name = fields.next()?.trim().to_string();
            let author_date = fields.next()?.trim();

            let timestamp = chrono::DateTime::parse_from_rfc3339(author_date)
                .ok()?
                .timestamp_millis();

            Some(WeeklyReportCommit {
                hash,
                short_hash,
                subject,
                author_name,
                timestamp,
            })
        })
        .collect()
}

fn ms_to_rfc3339(ms: i64) -> Result<String, String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|datetime| datetime.to_rfc3339())
        .ok_or_else(|| format!("时间格式不正确: {ms}"))
}

// ===== AI 润色 =====

/// 用默认模型润色周报草稿。一次性返回完整文本
pub async fn polish_report(
    store: &LocalJsonStore,
    input: &PolishWeeklyReportInput,
) -> Result<PolishWeeklyReportResult, String> {
    let draft = input.draft.trim();
    if draft.is_empty() {
        return Err("周报草稿为空，请先生成聚合草稿".to_string());
    }

    let target = {
        let default_repo = DefaultModelSettingRepository::new(store);
        let provider_repo = ProviderSettingRepository::new(store);
        crate::commands::api_generate::resolve_generation_target(
            &default_repo.list()?,
            &provider_repo.list()?,
            WEEKLY_REPORT_POLISH_MODEL_KEY,
            "请先在“默认模型”中配置周报润色模型，配置后即可使用 AI 润色",
        )?
    };

    let user_prompt = format!(
        "周报标题：{}\n\n周报草稿：\n{}",
        if input.title.trim().is_empty() {
            "（未提供）"
        } else {
            input.title.trim()
        },
        draft
    );

    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let response = client
        .completion_model(&target.model_id)
        .completion_request(rig::completion::Message::user(user_prompt))
        .preamble(POLISH_SYSTEM_PROMPT.to_string())
        .send()
        .await
        .map_err(|error| format!("模型请求失败: {error}"))?;

    let raw = extract_text(response.choice).ok_or_else(|| "模型未返回文本内容".to_string())?;
    let content = strip_code_fence(&raw);

    Ok(PolishWeeklyReportResult {
        content,
        model_label: target.label(),
    })
}

/// 从模型响应中提取全部文本片段
fn extract_text(choice: rig::OneOrMany<rig::completion::AssistantContent>) -> Option<String> {
    let text = choice
        .into_iter()
        .filter_map(|content| match content {
            rig::completion::AssistantContent::Text(text) => Some(text.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// 模型有时会把 Markdown 包进代码块，润色结果同样剥掉围栏
fn strip_code_fence(raw: &str) -> String {
    let trimmed = raw.trim();

    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let without_open = trimmed.trim_start_matches('`');
    let body = match without_open.find('\n') {
        Some(index) => &without_open[index + 1..],
        None => return trimmed.to_string(),
    };

    body.trim_end().trim_end_matches('`').trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_output_reads_all_fields_and_skips_blank_lines() {
        let raw = "abc123abc123abc123abc123abc123abc123\x1fabc12ef\x1ffeat(mail): 支持模板\x1f张三\x1f2026-09-30T10:00:00+08:00\n\n\
                   def456def456def456def456def456def456\x1fdef456\x1ffix(api): 修复空指针\x1f李四\x1f2026-10-01T09:30:00Z\n";

        let commits = parse_log_output(raw);
        assert_eq!(commits.len(), 2);

        assert_eq!(commits[0].hash, "abc123abc123abc123abc123abc123abc123");
        assert_eq!(commits[0].short_hash, "abc12ef");
        assert_eq!(commits[0].subject, "feat(mail): 支持模板");
        assert_eq!(commits[0].author_name, "张三");
        assert_eq!(commits[0].timestamp, 1_790_733_600_000);

        assert_eq!(commits[1].subject, "fix(api): 修复空指针");
        assert_eq!(commits[1].timestamp, 1_790_847_000_000);
    }

    #[test]
    fn parse_log_output_returns_empty_for_empty_input() {
        assert!(parse_log_output("").is_empty());
        assert!(parse_log_output("\n").is_empty());
    }

    #[test]
    fn ms_to_rfc3339_converts_and_validates() {
        assert_eq!(
            ms_to_rfc3339(0).expect("zero"),
            "1970-01-01T00:00:00+00:00"
        );
        assert!(ms_to_rfc3339(i64::MIN).is_err());
    }

    #[test]
    fn strip_code_fence_removes_markdown_wrapping() {
        let fenced = "```markdown\n# 周报\n正文\n```";
        assert_eq!(strip_code_fence(fenced), "# 周报\n正文");

        let bare = "# 周报";
        assert_eq!(strip_code_fence(bare), bare);
    }
}

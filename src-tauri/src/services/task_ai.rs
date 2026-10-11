//! 任务 AI 服务。
//!
//! 两个一次性调用（全项目 AI 功能均不流式）：
//! - `parse_task_text`：把一句话整理成结构化任务草稿（JSON 输出）
//! - `suggest_task_plan`：根据未完成任务给出简短的今日建议（纯文本输出）

use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;
use serde_json::json;

use crate::commands::api_generate;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::task::{
    normalize_due_date, normalize_priority, Task, TaskDraft, TASK_ASSISTANT_MODEL_KEY,
    TASK_STATUS_DONE,
};

/// 未完成任务清单注入 prompt 的截断上限（字符）
const OPEN_TASKS_CHARS: usize = 20_000;

const TASK_PARSE_PROMPT: &str = "你是 Eidolon 的任务整理助手。用户会用一句话描述一件要做的事，你把它整理成结构化任务。\n\n\
规则：\n\
- 只输出一个 JSON 对象本身，不要解释、不要 markdown 代码块。\n\
- 格式：{\"title\": \"任务标题\", \"priority\": \"high|medium|low|none\", \"due_date\": \"YYYY-MM-DD 或 null\"}\n\
- title：简洁的动宾短语标题，去掉口语和语气词，保留关键信息。\n\
- priority：从描述推断。明确说了高优先/紧急/重要用 high，说了中/一般用 medium，说了低/不急/有空再说用 low，没提用 none。\n\
- due_date：从相对时间（明天、周五、下周三、月底）结合今天日期推算；无法确定就用 null，不要猜。";

const TASK_PLAN_PROMPT: &str = "你是 Eidolon 的任务规划助手。根据用户当前的任务清单给出一份简短的今日建议。\n\n\
规则：\n\
- 直接输出建议正文，不要标题、不要开场白和客套话。\n\
- 用 2 到 4 条短句，每条一行、以 - 开头：先点出最该先做的事和原因（结合截止日期与优先级），再提醒今天到期或已经逾期的任务。\n\
- 清单里没有截止日期信息时，就按优先级给建议。\n\
- 总长度不超过 150 字。";

/// 把一句话整理成任务草稿
pub async fn parse_task_text(store: &LocalJsonStore, input: &str) -> Result<TaskDraft, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("请先输入任务描述".to_string());
    }

    let target = resolve_task_model(
        store,
        "请先在「应用设置 → 默认模型」中配置任务助手模型，配置后即可用 AI 整理任务",
    )?;

    let today = chrono::Local::now().format("%Y-%m-%d");
    let preamble = format!("{TASK_PARSE_PROMPT}\n\n今天是 {today}。");

    let raw = complete(&target, &preamble, input).await?;
    let content = api_generate::strip_code_fence(&raw);

    let draft = parse_draft_json(&content)?;

    let title = draft.title.trim().to_string();
    if title.is_empty() {
        return Err("模型没有整理出任务标题，请换个说法再试".to_string());
    }

    Ok(TaskDraft {
        title,
        priority: normalize_priority(&draft.priority),
        due_date: normalize_due_date(&draft.due_date).unwrap_or_default(),
    })
}

/// 根据未完成任务给出今日建议（纯文本）
pub async fn suggest_task_plan(store: &LocalJsonStore, tasks: &[Task]) -> Result<String, String> {
    let open_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.status != TASK_STATUS_DONE)
        .collect();

    if open_tasks.is_empty() {
        return Err("当前没有未完成的任务".to_string());
    }

    let target = resolve_task_model(
        store,
        "请先在「应用设置 → 默认模型」中配置任务助手模型，配置后即可生成今日建议",
    )?;

    let today = chrono::Local::now().format("%Y-%m-%d");
    let preamble = format!("{TASK_PLAN_PROMPT}\n\n今天是 {today}。");

    let listing: Vec<_> = open_tasks
        .iter()
        .map(|task| {
            json!({
                "title": task.title,
                "status": task.status,
                "priority": task.priority,
                "due_date": task.due_date,
            })
        })
        .collect();
    let listing_text = serde_json::to_string_pretty(&listing).unwrap_or_default();
    let listing_text = truncate_chars(&listing_text, OPEN_TASKS_CHARS)
        .unwrap_or_else(|| "（任务清单过长，已省略）".to_string());

    complete(&target, &preamble, &listing_text).await
}

fn resolve_task_model(store: &LocalJsonStore, missing_message: &str) -> Result<crate::commands::api_generate::GenerationTarget, String> {
    let default_repo = DefaultModelSettingRepository::new(store);
    let provider_repo = ProviderSettingRepository::new(store);
    api_generate::resolve_generation_target(
        &default_repo.list()?,
        &provider_repo.list()?,
        TASK_ASSISTANT_MODEL_KEY,
        missing_message,
    )
}

/// 一次性补全调用：preamble 为系统提示，user_message 为用户输入
async fn complete(
    target: &crate::commands::api_generate::GenerationTarget,
    preamble: &str,
    user_message: &str,
) -> Result<String, String> {
    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let mut request = client
        .completion_model(&target.model_id)
        .completion_request(RigMessage::user(user_message))
        .preamble(preamble.to_string());

    if let Some(temperature) = target.temperature {
        request = request.temperature(temperature);
    }
    if let Some(max_tokens) = target.max_tokens {
        request = request.max_tokens(max_tokens);
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("模型请求失败: {error}"))?;

    extract_text(response.choice).ok_or_else(|| "模型未返回可展示的文本内容".to_string())
}

/// 从模型输出里截取第一个 `{` 到最后一个 `}` 再解析 JSON
fn parse_draft_json(raw: &str) -> Result<TaskDraft, String> {
    let start = raw.find('{').ok_or("模型没有返回任务信息")?;
    let end = raw.rfind('}').ok_or("模型没有返回任务信息")?;
    if end < start {
        return Err("模型返回的任务信息无法解析".to_string());
    }

    serde_json::from_str::<TaskDraft>(&raw[start..=end])
        .map_err(|error| format!("模型返回的任务信息无法解析: {error}"))
}

/// 按字符数截断，超出时补省略号；内容为空时返回 None
fn truncate_chars(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut chars = trimmed.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        Some(format!("{head}…"))
    } else {
        Some(head)
    }
}

fn extract_text(choice: rig::OneOrMany<AssistantContent>) -> Option<String> {
    let text = choice
        .into_iter()
        .filter_map(|content| match content {
            AssistantContent::Text(text) => Some(text.text),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_draft_json_tolerates_prose_and_fence_remainder() {
        let raw = "好的，整理如下：{\"title\": \"写周报\", \"priority\": \"high\", \"due_date\": \"2026-10-16\"} 以上。";
        let draft = parse_draft_json(raw).expect("draft should parse");
        assert_eq!(draft.title, "写周报");
        assert_eq!(draft.priority, "high");
        assert_eq!(draft.due_date, "2026-10-16");
    }

    #[test]
    fn parse_draft_json_rejects_empty_output() {
        assert!(parse_draft_json("没有 JSON").is_err());
        assert!(parse_draft_json("{ broken").is_err());
    }

    #[test]
    fn truncate_chars_limits_by_chars_not_bytes() {
        let long = "一二三四五".to_string();
        assert_eq!(truncate_chars(&long, 3).as_deref(), Some("一二三…"));
        assert_eq!(truncate_chars("abc", 5).as_deref(), Some("abc"));
        assert_eq!(truncate_chars("   ", 5), None);
    }
}

//! 任务 AI 服务。
//!
//! 两个一次性调用（全项目 AI 功能均不流式）：
//! - `parse_task_text`：把一句话整理成结构化任务草稿（JSON 输出）
//! - `suggest_task_plan`：把未排期任务排成结构化时间块（JSON 输出，Rust 侧校验）

use std::collections::HashMap;

use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;
use serde::Deserialize;
use serde_json::json;

use crate::commands::api_generate;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::task::{
    normalize_due_date, normalize_priority, Task, TaskDraft, TaskPlan, TaskPlanBlock,
    TASK_ASSISTANT_MODEL_KEY, TASK_STATUS_DONE,
};

const TASK_PARSE_PROMPT: &str = "你是 Eidolon 的任务整理助手。用户会用一句话描述一件要做的事，你把它整理成结构化任务。\n\n\
规则：\n\
- 只输出一个 JSON 对象本身，不要解释、不要 markdown 代码块。\n\
- 格式：{\"title\": \"任务标题\", \"priority\": \"high|medium|low|none\", \"due_date\": \"YYYY-MM-DD 或 null\"}\n\
- title：简洁的动宾短语标题，去掉口语和语气词，保留关键信息。\n\
- priority：从描述推断。明确说了高优先/紧急/重要用 high，说了中/一般用 medium，说了低/不急/有空再说用 low，没提用 none。\n\
- due_date：从相对时间（明天、周五、下周三、月底）结合今天日期推算；无法确定就用 null，不要猜。";

const TASK_PLAN_PROMPT: &str = "你是 Eidolon 的任务排期助手。把用户给出的未排期任务安排到未来 7 天的时间块里。\n\n\
规则：\n\
- 只输出一个 JSON 对象本身，不要解释、不要 markdown 代码块。\n\
- 格式：{\"blocks\": [{\"task_id\": \"...\", \"date\": \"YYYY-MM-DD\", \"start\": \"HH:MM\", \"end\": \"HH:MM\"}], \"note\": \"一句安排思路\"}\n\
- blocks 里只能引用清单里给出的 task_id，不要编造任务；每个任务最多一个时间块；不重要的任务可以不排。\n\
- 时间约束：工作时段 09:00–18:00；避开 12:00–13:00 午休；不要与「已有安排」重叠；时长参考任务预估，没预估的按 30–60 分钟估。\n\
- 优先安排：今天或已逾期的任务、高优先级、截止日近的任务。\n\
- note：不超过 40 字，说明安排思路（例如：今天先清逾期，周报放周二上午）。";

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

/// 把未排期任务排成结构化时间块（JSON 输出，Rust 侧逐块校验）
pub async fn suggest_task_plan(store: &LocalJsonStore, tasks: &[Task]) -> Result<TaskPlan, String> {
    let candidates: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.status != TASK_STATUS_DONE && task.start_at.trim().is_empty())
        .collect();

    if candidates.is_empty() {
        return Err("当前没有需要排期的任务".to_string());
    }

    let target = resolve_task_model(
        store,
        "请先在「应用设置 → 默认模型」中配置任务助手模型，配置后即可生成排期建议",
    )?;

    let today = chrono::Local::now().date_naive();
    let today_text = today.format("%Y-%m-%d").to_string();
    let horizon = today + chrono::Duration::days(7);
    let preamble = format!("{TASK_PLAN_PROMPT}\n\n今天是 {today_text}，最多排到 {}。", horizon.format("%Y-%m-%d"));

    // 待排任务 + 已有安排（作为避让上下文，AI 不重排它们）
    let listing: Vec<_> = candidates
        .iter()
        .map(|task| {
            json!({
                "task_id": task.id,
                "title": task.title,
                "status": task.status,
                "priority": task.priority,
                "due_date": task.due_date,
                "estimate_minutes": task.estimate_minutes,
            })
        })
        .collect();
    let scheduled: Vec<_> = tasks
        .iter()
        .filter(|task| task.status != TASK_STATUS_DONE && !task.start_at.trim().is_empty())
        .map(|task| {
            json!({
                "title": task.title,
                "start_at": task.start_at,
                "end_at": task.end_at,
            })
        })
        .collect();

    let user_message = format!(
        "未排期任务：\n{}\n\n已有安排（不要与之重叠）：\n{}",
        serde_json::to_string_pretty(&listing).unwrap_or_default(),
        if scheduled.is_empty() {
            "（无）".to_string()
        } else {
            serde_json::to_string_pretty(&scheduled).unwrap_or_default()
        },
    );

    let raw = complete(&target, &preamble, &user_message).await?;
    let content = api_generate::strip_code_fence(&raw);

    build_plan(&content, &candidates, today, horizon)
}

/// 解析并校验模型返回的排期 JSON：
/// task_id 必须在候选集内、日期落在 [today, horizon]、start < end；坏块丢弃，全坏则报错
fn build_plan(raw: &str, candidates: &[&Task], today: chrono::NaiveDate, horizon: chrono::NaiveDate) -> Result<TaskPlan, String> {
    #[derive(Deserialize)]
    struct PlanBlockRaw {
        task_id: String,
        date: String,
        start: String,
        end: String,
    }

    #[derive(Deserialize)]
    struct PlanRaw {
        #[serde(default)]
        blocks: Vec<PlanBlockRaw>,
        #[serde(default)]
        note: String,
    }

    let start = raw.find('{').ok_or("模型没有返回排期信息")?;
    let end = raw.rfind('}').ok_or("模型没有返回排期信息")?;
    if end < start {
        return Err("模型返回的排期信息无法解析".to_string());
    }

    let plan: PlanRaw = serde_json::from_str(&raw[start..=end])
        .map_err(|error| format!("模型返回的排期信息无法解析: {error}"))?;

    let by_id: HashMap<&str, &Task> = candidates
        .iter()
        .map(|task| (task.id.as_str(), *task))
        .collect();

    let mut blocks: Vec<TaskPlanBlock> = Vec::new();
    for block in plan.blocks {
        let Some(task) = by_id.get(block.task_id.trim()) else {
            continue; // 编造的任务直接丢弃
        };

        let Some(date) = chrono::NaiveDate::parse_from_str(block.date.trim(), "%Y-%m-%d").ok() else {
            continue;
        };
        if date < today || date > horizon {
            continue;
        }

        let parse_time = |text: &str| chrono::NaiveTime::parse_from_str(text.trim(), "%H:%M").ok();
        let (Some(start_time), Some(end_time)) = (parse_time(&block.start), parse_time(&block.end))
        else {
            continue;
        };
        if end_time <= start_time {
            continue;
        }

        let date_text = date.format("%Y-%m-%d").to_string();
        blocks.push(TaskPlanBlock {
            task_id: task.id.clone(),
            title: task.title.clone(),
            priority: task.priority.clone(),
            start_at: format!("{date_text}T{}", start_time.format("%H:%M")),
            end_at: format!("{date_text}T{}", end_time.format("%H:%M")),
        });
    }

    if blocks.is_empty() {
        return Err("模型没有返回可用的时间安排，请重试".to_string());
    }

    Ok(TaskPlan {
        blocks,
        note: plan.note.trim().to_string(),
    })
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
        .map_err(|error| crate::services::model_error::describe_completion_error(&error))?;

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

    fn plan_candidates() -> Vec<Task> {
        vec![
            Task {
                id: "task_a".to_string(),
                title: "写周报".to_string(),
                priority: "high".to_string(),
                ..Default::default()
            },
            Task {
                id: "task_b".to_string(),
                title: "回复评审".to_string(),
                ..Default::default()
            },
        ]
    }

    fn plan_horizon() -> (chrono::NaiveDate, chrono::NaiveDate) {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 13).expect("monday");
        (today, today + chrono::Duration::days(7))
    }

    #[test]
    fn build_plan_maps_blocks_back_to_tasks() {
        let (today, horizon) = plan_horizon();
        let tasks = plan_candidates();
        let candidates: Vec<_> = tasks.iter().collect();
        let raw = r#"{"blocks": [
            {"task_id": "task_a", "date": "2026-10-14", "start": "09:00", "end": "10:30"},
            {"task_id": "task_b", "date": "2026-10-13", "start": "14:00", "end": "15:00"}
        ], "note": " 先清周报 "}"#;

        let plan = build_plan(raw, &candidates, today, horizon).expect("plan should build");
        assert_eq!(plan.note, "先清周报");
        assert_eq!(plan.blocks.len(), 2);

        let first = &plan.blocks[0];
        assert_eq!(first.task_id, "task_a");
        assert_eq!(first.title, "写周报", "title comes from the task, not the model");
        assert_eq!(first.priority, "high");
        assert_eq!(first.start_at, "2026-10-14T09:00");
        assert_eq!(first.end_at, "2026-10-14T10:30");
    }

    #[test]
    fn build_plan_drops_invalid_blocks() {
        let (today, horizon) = plan_horizon();
        let tasks = plan_candidates();
        let candidates: Vec<_> = tasks.iter().collect();
        let raw = r#"{"blocks": [
            {"task_id": "task_missing", "date": "2026-10-14", "start": "09:00", "end": "10:00"},
            {"task_id": "task_a", "date": "2026-11-01", "start": "09:00", "end": "10:00"},
            {"task_id": "task_a", "date": "10/14", "start": "09:00", "end": "10:00"},
            {"task_id": "task_a", "date": "2026-10-14", "start": "9点", "end": "10:00"},
            {"task_id": "task_a", "date": "2026-10-14", "start": "11:00", "end": "10:00"},
            {"task_id": "task_b", "date": "2026-10-15", "start": "09:00", "end": "09:30"}
        ]}"#;

        let plan = build_plan(raw, &candidates, today, horizon).expect("plan should build");
        assert_eq!(plan.blocks.len(), 1, "only the valid block survives");
        assert_eq!(plan.blocks[0].task_id, "task_b");
    }

    #[test]
    fn build_plan_rejects_when_nothing_valid() {
        let (today, horizon) = plan_horizon();
        let tasks = plan_candidates();
        let candidates: Vec<_> = tasks.iter().collect();
        let raw = r#"{"blocks": [], "note": "空"}"#;
        assert!(build_plan(raw, &candidates, today, horizon).is_err());
        assert!(build_plan("没有 JSON", &candidates, today, horizon).is_err());
    }
}

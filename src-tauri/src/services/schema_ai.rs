//! 数据字典 AI 服务。
//!
//! 两个一次性调用（全项目 AI 功能均不流式）：
//! - `interpret_changes`：把快照间变更翻译成人话
//! - `answer_question`：针对整库表结构问答

use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;

use crate::commands::api_generate;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::schema_dict::{SchemaChange, TimelineEntry, SCHEMA_ASSISTANT_MODEL_KEY};

/// 表结构 JSON 注入 prompt 的截断上限（字符），大库可能几百张表
const SCHEMA_CHARS: usize = 60_000;
/// 变更 JSON 注入 prompt 的截断上限（字符）
const CHANGES_CHARS: usize = 20_000;

const INTERPRET_PROMPT: &str = "你是 Eidolon 的数据库结构助手。用户给你两个版本表结构快照之间的变更列表，请用中文给一段简短解读。\n\n\
规则：\n\
- 直接输出解读正文，不要标题、不要客套。\n\
- 先一句话概括这次变更的主题（如主要新增了什么业务相关的表），再挑重点列 2 到 4 条：删表/删字段要特别提醒。\n\
- 变更列表为空就说明两版结构一致。\n\
- 总长度不超过 150 字。";

const QUESTION_PROMPT: &str = "你是 Eidolon 的数据字典助手。用户针对一个数据库的表结构提问题，请基于给定的表结构 JSON 用中文回答。\n\n\
规则：\n\
- 先直接回答，再点出依据（涉及的表名和关键字段）。\n\
- 引用表名和字段名时保留原名。\n\
- 表结构里找不到答案就直说，不要编造。\n\
- 回答控制在 200 字以内。\n\
- 如果给的结构被截断，说明只覆盖了部分表。";

/// 解读某快照相对上一版的变更
pub async fn interpret_changes(
    store: &LocalJsonStore,
    before_label: &str,
    entry: &TimelineEntry,
    changes: &[SchemaChange],
) -> Result<String, String> {
    let target = resolve_schema_model(
        store,
        "请先在「应用设置 → 默认模型」中配置数据字典助手模型，配置后即可解读变更",
    )?;

    let changes_json = serde_json::to_string_pretty(changes)
        .map_err(|error| format!("序列化变更失败: {error}"))?;
    let changes_json = truncate_chars(&changes_json, CHANGES_CHARS)
        .unwrap_or_else(|| "[]".to_string());

    let today = chrono::Local::now().format("%Y-%m-%d");
    let preamble = format!(
        "{INTERPRET_PROMPT}\n\n今天是 {today}。当前快照叫「{label}」，上一个快照叫「{before_label}」。",
        label = entry.label
    );
    let user = format!("变更列表 JSON：\n{changes_json}");

    complete(&target, &preamble, &user).await
}

/// 针对最新快照的表结构问答
pub async fn answer_question(
    store: &LocalJsonStore,
    database_name: &str,
    tables_json: &str,
    question: &str,
) -> Result<String, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("请先输入问题".to_string());
    }

    let target = resolve_schema_model(
        store,
        "请先在「应用设置 → 默认模型」中配置数据字典助手模型，配置后即可提问",
    )?;

    let truncated = truncate_chars(tables_json, SCHEMA_CHARS);
    let schema_block = match truncated {
        Some(text) => {
            if tables_json.chars().count() > SCHEMA_CHARS {
                format!("{text}\n（结构过长已截断，仅覆盖部分表）")
            } else {
                text
            }
        }
        None => return Err("当前快照没有表结构数据".to_string()),
    };

    let preamble = format!(
        "{QUESTION_PROMPT}\n\n数据库叫「{name}」。以下是表结构 JSON：\n{schema_block}",
        name = database_name
    );

    complete(&target, &preamble, question).await
}

fn resolve_schema_model(
    store: &LocalJsonStore,
    missing_message: &str,
) -> Result<crate::commands::api_generate::GenerationTarget, String> {
    let default_repo = DefaultModelSettingRepository::new(store);
    let provider_repo = ProviderSettingRepository::new(store);
    api_generate::resolve_generation_target(
        &default_repo.list()?,
        &provider_repo.list()?,
        SCHEMA_ASSISTANT_MODEL_KEY,
        missing_message,
    )
}

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
    fn truncate_chars_limits_by_chars_not_bytes() {
        let long = "一二三四五".to_string();
        assert_eq!(truncate_chars(&long, 3).as_deref(), Some("一二三…"));
        assert_eq!(truncate_chars("abc", 5).as_deref(), Some("abc"));
        assert_eq!(truncate_chars("   ", 5), None);
    }
}

//! 收藏夹 AI 服务。
//!
//! 两个一次性调用（全项目 AI 功能均不流式）：
//! - `summarize_link`：现场抓网页正文并总结（不落库）
//! - `answer_question`：基于收藏条目上下文问答

use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;

use crate::commands::api_generate;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::collect::{CollectItem, COLLECT_ASSISTANT_MODEL_KEY, COLLECT_KIND_LINK};
use crate::services::collect_fetch;

/// 收藏条目上下文注入 prompt 的截断上限（字符）
const CONTEXT_CHARS: usize = 60_000;
/// 单条条目在上下文里的截断上限（字符），条目多时保证每条都有露出
const ITEM_CHARS: usize = 2_000;

const SUMMARY_PROMPT: &str = "你是 Eidolon 的收藏夹助手。用户收藏了一个网页并给你页面正文，请用中文输出这份内容的摘要。\n\n\
输出格式：\n\
- 先一句话概括这篇内容讲了什么。\n\
- 再用 markdown 无序列表给出 3 到 5 个要点。\n\n\
规则：\n\
- 只根据给出的正文总结，正文里没有的不要编造。\n\
- 直接输出摘要，不要开场白和标题。";

const QUESTION_PROMPT: &str = "你是 Eidolon 的收藏夹助手。用户针对自己的收藏提问，请基于给定的收藏条目列表用中文回答。\n\n\
规则：\n\
- 先直接回答问题。\n\
- 再列出依据：引用相关条目的标题，链接类条目附上 URL。\n\
- 收藏里找不到答案就直说，不要编造条目。\n\
- 如果条目列表被截断，说明只覆盖了较新的部分收藏。";

/// 单条链接摘要：现场抓正文 → 总结（不落库）
pub async fn summarize_link(store: &LocalJsonStore, url: &str) -> Result<String, String> {
    let target = resolve_collect_model(
        store,
        "请先在「应用设置 → 默认模型」中配置收藏夹助手模型，配置后即可生成 AI 摘要",
    )?;

    let article = collect_fetch::fetch_page_text(url).await?;
    let user = format!("网页地址：{url}\n\n正文：\n{article}");

    complete(&target, SUMMARY_PROMPT, &user).await
}

/// 收藏全库问答：上下文由命令层组装，服务层负责截断与 prompt
pub async fn answer_question(
    store: &LocalJsonStore,
    context: &str,
    question: &str,
) -> Result<String, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("请先输入问题".to_string());
    }

    let target = resolve_collect_model(
        store,
        "请先在「应用设置 → 默认模型」中配置收藏夹助手模型，配置后即可提问",
    )?;

    let truncated = truncate_chars(context, CONTEXT_CHARS);
    let context_block = match truncated {
        Some(text) => {
            if context.chars().count() > CONTEXT_CHARS {
                format!("{text}\n（收藏过多已截断，仅覆盖较新的条目）")
            } else {
                text
            }
        }
        None => return Err("收藏夹还没有内容".to_string()),
    };

    let preamble = format!("{QUESTION_PROMPT}\n\n以下是收藏条目列表（每行一条）：\n{context_block}");
    complete(&target, &preamble, question).await
}

/// 把收藏条目拼成 LLM 上下文行（纯函数，可单测）
pub(crate) fn build_context_lines(items: &[CollectItem]) -> String {
    items
        .iter()
        .map(item_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn item_line(item: &CollectItem) -> String {
    let archived_mark = if item.archived { "【已归档】" } else { "" };
    let body = if item.kind == COLLECT_KIND_LINK {
        let title = item.title.as_deref().unwrap_or("（未获取标题）");
        let url = item.url.as_deref().unwrap_or("");
        let mut line = format!("[链接] {title} | {url}");
        if let Some(description) = item.description.as_deref().filter(|text| !text.is_empty()) {
            line.push_str(&format!(" | 摘要：{description}"));
        }
        let remark = item.content.trim();
        if !remark.is_empty() {
            line.push_str(&format!(" | 备注：{remark}"));
        }
        line
    } else {
        format!("[记事] {}", item.content.trim())
    };

    truncate_chars(&format!("{archived_mark}{body}"), ITEM_CHARS).unwrap_or_default()
}

fn resolve_collect_model(
    store: &LocalJsonStore,
    missing_message: &str,
) -> Result<crate::commands::api_generate::GenerationTarget, String> {
    let default_repo = DefaultModelSettingRepository::new(store);
    let provider_repo = ProviderSettingRepository::new(store);
    api_generate::resolve_generation_target(
        &default_repo.list()?,
        &provider_repo.list()?,
        COLLECT_ASSISTANT_MODEL_KEY,
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

    fn link(title: Option<&str>, url: &str) -> CollectItem {
        CollectItem {
            kind: "link".to_string(),
            title: title.map(str::to_string),
            url: Some(url.to_string()),
            ..Default::default()
        }
    }

    fn note(content: &str) -> CollectItem {
        CollectItem {
            kind: "note".to_string(),
            content: content.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_context_lines_formats_links_and_notes() {
        let items = vec![
            CollectItem {
                description: Some("一篇讲正文提取的文章".to_string()),
                content: "回头细读".to_string(),
                ..link(Some("正文提取入门"), "https://example.com/a")
            },
            note("周五前交周报"),
        ];

        let context = build_context_lines(&items);
        assert_eq!(
            context,
            "[链接] 正文提取入门 | https://example.com/a | 摘要：一篇讲正文提取的文章 | 备注：回头细读\n\
             [记事] 周五前交周报"
        );
    }

    #[test]
    fn build_context_lines_marks_archived_and_missing_title() {
        let items = vec![CollectItem {
            archived: true,
            ..link(None, "https://example.com/b")
        }];

        let context = build_context_lines(&items);
        assert_eq!(context, "【已归档】[链接] （未获取标题） | https://example.com/b");
    }

    #[test]
    fn build_context_lines_truncates_overlong_items() {
        let item = note(&"记".repeat(ITEM_CHARS + 100));

        let context = build_context_lines(std::slice::from_ref(&item));
        let line = context.lines().next().expect("one line");
        assert_eq!(line.chars().count(), ITEM_CHARS + 1, "ITEM_CHARS plus ellipsis");
        assert!(line.ends_with('…'));
    }

    #[test]
    fn truncate_chars_limits_by_chars_not_bytes() {
        let long = "一二三四五".to_string();
        assert_eq!(truncate_chars(&long, 3).as_deref(), Some("一二三…"));
        assert_eq!(truncate_chars("abc", 5).as_deref(), Some("abc"));
        assert_eq!(truncate_chars("   ", 5), None);
    }
}

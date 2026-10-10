use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;

use crate::commands::api_generate;
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::docs::{DocsAiChatMessage, DocsAiChatResult};

pub const DOCS_AI_MODEL_KEY: &str = "docs_chat";

/// 每轮最多携带的历史消息条数（防止上下文膨胀）
const MAX_HISTORY_MESSAGES: usize = 8;
/// 单条历史消息的截断长度（字符）
const HISTORY_MESSAGE_CHARS: usize = 4000;
/// 文档全文注入 preamble 的截断上限（字符）
const DOCUMENT_CHARS: usize = 60_000;

const DOCS_AI_PROMPT: &str = "你是 Eidolon 文档编辑助手，帮用户打磨手边这篇 Markdown 文档。\n\n\
规则：\n\
- 默认中文，简洁直接，先结论后细节。\n\
- 用户请你修改文档时：先用一两句话说明改了什么，然后把完整修改后的文档全文放进一个 eidolon-doc 围栏代码块输出。\n\
- 围栏必须用四个反引号开头（````eidolon-doc）并以同样四个反引号结束，因为文档内部可能包含普通代码块。\n\
- 围栏内必须是修改后的完整文档，不允许用「其余不变」「……」等任何方式省略内容。\n\
- 只是提问、讨论、总结时正常回答，不要输出该围栏。\n\
- 尊重文档既有风格：标题层级、列表写法、表格与 mermaid 代码块原样保留，除非用户明确要求调整。\n\
- 文档为空时帮用户从零起草，同样用围栏输出全文。";

/// 执行一轮文档助手对话：解析模型 → 注入文档全文 → 回答。
/// 会话历史由前端持有，本服务无状态。
pub async fn run_docs_ai_turn(
    store: &LocalJsonStore,
    history: &[DocsAiChatMessage],
    document: &str,
    message: &str,
) -> Result<DocsAiChatResult, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("消息内容不能为空".to_string());
    }

    let target = {
        let default_repo = DefaultModelSettingRepository::new(store);
        let provider_repo = ProviderSettingRepository::new(store);
        api_generate::resolve_generation_target(
            &default_repo.list()?,
            &provider_repo.list()?,
            DOCS_AI_MODEL_KEY,
            "请先在「应用设置 → 默认模型」中配置文档助手模型，配置后即可对话",
        )?
    };

    let document_block = if document.trim().is_empty() {
        "（当前是空文档）".to_string()
    } else {
        truncate_chars(document, DOCUMENT_CHARS)
            .map(|text| {
                if document.chars().count() > DOCUMENT_CHARS {
                    format!("{text}\n\n（文档过长，已截断，仅基于以上内容回答）")
                } else {
                    text
                }
            })
            .unwrap_or_else(|| "（当前是空文档）".to_string())
    };
    let preamble = format!("{DOCS_AI_PROMPT}\n\n## 当前文档全文\n{document_block}");

    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let mut request = client
        .completion_model(&target.model_id)
        .completion_request(RigMessage::user(message))
        .preamble(preamble);

    for message in history.iter().rev().take(MAX_HISTORY_MESSAGES).rev() {
        let Some(content) = truncate_chars(&message.content, HISTORY_MESSAGE_CHARS) else {
            continue;
        };
        let rig_message = match message.role.as_str() {
            "user" => RigMessage::user(content),
            "assistant" => RigMessage::assistant(content),
            _ => continue,
        };
        request = request.message(rig_message);
    }

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

    let answer = extract_text(response.choice)
        .ok_or_else(|| "模型未返回可展示的文本内容".to_string())?;

    Ok(DocsAiChatResult {
        content: answer,
        model_label: target.label(),
    })
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

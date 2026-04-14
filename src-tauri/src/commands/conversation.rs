use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::default_model::DefaultModelSetting;
use crate::models::model_config::ProviderSetting;
use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationReply {
    pub content: String,
}

#[derive(Debug, Clone)]
struct ChatTarget {
    model_id: String,
    api_key: String,
    base_url: String,
    temperature: Option<f64>,
    max_tokens: Option<u64>,
    additional_params: Option<Value>,
}

#[tauri::command]
pub async fn send_conversation_message(
    store: tauri::State<'_, LocalJsonStore>,
    messages: Vec<ConversationMessageInput>,
) -> Result<ConversationReply, String> {
    let default_repo = DefaultModelSettingRepository::new(&store);
    let provider_repo = ProviderSettingRepository::new(&store);

    let default_settings = default_repo.list()?;
    let provider_settings = provider_repo.list()?;
    let target: ChatTarget = resolve_chat_target(&default_settings, &provider_settings)?;
    let rig_messages = build_rig_messages(&messages)?;

    let last_message = rig_messages
        .last()
        .cloned()
        .ok_or_else(|| "消息不能为空".to_string())?;

    let prompt = match last_message {
        RigMessage::User { .. } => last_message,
        _ => return Err("最后一条消息必须是用户消息".to_string()),
    };

    let history = rig_messages
        .into_iter()
        .take(messages.len().saturating_sub(1))
        .collect::<Vec<_>>();

    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let mut request = client
        .completion_model(&target.model_id)
        .completion_request(prompt);

    for message in history {
        request = request.message(message);
    }

    if let Some(temperature) = target.temperature {
        request = request.temperature(temperature);
    }

    if let Some(max_tokens) = target.max_tokens {
        request = request.max_tokens(max_tokens);
    }

    if let Some(additional_params) = target.additional_params {
        request = request.additional_params(additional_params);
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("模型请求失败: {error}"))?;

    let content = extract_text_response(response.choice)
        .ok_or_else(|| "模型未返回可展示的文本内容".to_string())?;

    Ok(ConversationReply { content })
}

fn resolve_chat_target(
    default_settings: &[DefaultModelSetting],
    provider_settings: &[ProviderSetting],
) -> Result<ChatTarget, String> {
    let default_setting = default_settings
        .iter()
        .find(|setting| setting.key == "assistant")
        .ok_or_else(|| "请先在“默认模型”中配置默认模型".to_string())?;

    let provider_setting = provider_settings
        .iter()
        .find(|setting| setting.provider_id == default_setting.provider_id)
        .ok_or_else(|| format!("未找到 {} 的模型服务配置", default_setting.provider_id))?;

    if !provider_setting.enabled {
        return Err(format!("{} 已被禁用，请先启用后再对话", provider_setting.provider_id));
    }

    if provider_setting.api_key.trim().is_empty() {
        return Err(format!("{} 的 API Key 不能为空", provider_setting.provider_id));
    }

    if provider_setting.base_url.trim().is_empty() {
        return Err(format!("{} 的 Base URL 不能为空", provider_setting.provider_id));
    }

    Ok(ChatTarget {
        model_id: default_setting.model_id.clone(),
        api_key: provider_setting.api_key.clone(),
        base_url: provider_setting.base_url.clone(),
        temperature: parse_optional_f64(&default_setting.temperature, "temperature")?,
        max_tokens: parse_optional_u64(&default_setting.max_tokens, "max_tokens")?,
        additional_params: build_additional_params(default_setting)?,
    })
}

fn build_additional_params(setting: &DefaultModelSetting) -> Result<Option<Value>, String> {
    let mut params = Map::new();

    if let Some(top_p) = parse_optional_f64(&setting.top_p, "top_p")? {
        params.insert("top_p".to_string(), json!(top_p));
    }

    if let Some(presence_penalty) = parse_optional_f64(&setting.presence_penalty, "presence_penalty")? {
        params.insert("presence_penalty".to_string(), json!(presence_penalty));
    }

    if let Some(frequency_penalty) = parse_optional_f64(&setting.frequency_penalty, "frequency_penalty")? {
        params.insert("frequency_penalty".to_string(), json!(frequency_penalty));
    }

    if params.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(params)))
    }
}

fn parse_optional_f64(value: &str, field: &str) -> Result<Option<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("{field} 格式不正确"))
}

fn parse_optional_u64(value: &str, field: &str) -> Result<Option<u64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("{field} 格式不正确"))
}

fn build_rig_messages(messages: &[ConversationMessageInput]) -> Result<Vec<RigMessage>, String> {
    let mut results = Vec::with_capacity(messages.len());

    for message in messages {
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }

        let rig_message = match message.role.as_str() {
            "user" => RigMessage::user(content),
            "assistant" => RigMessage::assistant(content),
            "system" => RigMessage::system(content),
            other => return Err(format!("不支持的消息角色: {other}")),
        };

        results.push(rig_message);
    }

    if results.is_empty() {
        return Err("消息不能为空".to_string());
    }

    Ok(results)
}

fn extract_text_response(choice: rig::OneOrMany<AssistantContent>) -> Option<String> {
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
    use super::{
        build_rig_messages, resolve_chat_target, ConversationMessageInput,
    };
    use crate::models::default_model::DefaultModelSetting;
    use crate::models::model_config::ProviderSetting;

    fn assistant_default() -> DefaultModelSetting {
        DefaultModelSetting {
            key: "assistant".to_string(),
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
            temperature: "0.7".to_string(),
            top_p: "0.9".to_string(),
            max_tokens: "4096".to_string(),
            presence_penalty: "0".to_string(),
            frequency_penalty: "0".to_string(),
        }
    }

    fn deepseek_setting() -> ProviderSetting {
        ProviderSetting {
            provider_id: "deepseek".to_string(),
            enabled: true,
            api_key: "sk-test".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
        }
    }

    #[test]
    fn resolve_chat_target_requires_assistant_default_model() {
        let error = resolve_chat_target(&[], &[deepseek_setting()])
            .expect_err("assistant default model should be required");

        assert!(error.contains("默认模型"));
    }

    #[test]
    fn resolve_chat_target_requires_matching_provider_setting() {
        let error = resolve_chat_target(&[assistant_default()], &[])
            .expect_err("provider setting should be required");

        assert!(error.contains("模型服务配置"));
    }

    #[test]
    fn build_rig_messages_rejects_unknown_roles() {
        let error = build_rig_messages(&[ConversationMessageInput {
            role: "tool".to_string(),
            content: "hello".to_string(),
        }])
        .expect_err("unknown roles should be rejected");

        assert!(error.contains("不支持的消息角色"));
    }
}

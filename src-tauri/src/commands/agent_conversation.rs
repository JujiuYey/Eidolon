use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::agent_conversation::AgentConversationRepository;
use crate::db::repositories::agent_profile::AgentProfileRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::agent_conversation::{AgentConversation, AgentConversationMessage};
use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;

#[tauri::command]
pub fn list_agent_conversations(
    store: tauri::State<'_, LocalJsonStore>,
    agent_profile_id: String,
) -> Result<Vec<AgentConversation>, String> {
    let repo = AgentConversationRepository::new(&store);
    repo.list_by_agent(&agent_profile_id)
}

#[tauri::command]
pub fn list_recent_agent_conversations(
    store: tauri::State<'_, LocalJsonStore>,
    limit: Option<usize>,
) -> Result<Vec<AgentConversation>, String> {
    let repo = AgentConversationRepository::new(&store);
    repo.list_recent(limit)
}

#[tauri::command]
pub fn create_agent_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    agent_profile_id: String,
) -> Result<AgentConversation, String> {
    let profile_repo = AgentProfileRepository::new(&store);
    let profile = profile_repo
        .get(&agent_profile_id)?
        .ok_or_else(|| format!("未找到 id 为 {} 的 Agent", agent_profile_id))?;

    let repo = AgentConversationRepository::new(&store);
    repo.create_from_profile(&profile)
}

#[tauri::command]
pub fn get_agent_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
) -> Result<Option<AgentConversation>, String> {
    let repo = AgentConversationRepository::new(&store);
    repo.get(&conversation_id)
}

#[tauri::command]
pub fn delete_agent_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
) -> Result<String, String> {
    let repo = AgentConversationRepository::new(&store);
    repo.delete(&conversation_id)
}

#[tauri::command]
pub fn list_agent_conversation_messages(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
) -> Result<Vec<AgentConversationMessage>, String> {
    let repo = AgentConversationRepository::new(&store);
    repo.list_messages(&conversation_id)
}

#[tauri::command]
pub async fn send_agent_conversation_message(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
    content: String,
) -> Result<AgentConversationMessage, String> {
    let repo = AgentConversationRepository::new(&store);
    let conversation = repo
        .get(&conversation_id)?
        .ok_or_else(|| "未找到会话".to_string())?;

    // Append user message
    repo.append_user_message(&conversation.id, &content)?;

    // Get conversation history
    let history = repo.list_messages(&conversation.id)?;

    // Run agent turn with snapshot configuration
    let assistant_result = run_snapshot_agent_turn(&store, &conversation, &history).await;

    // Append assistant message (success or error)
    match assistant_result {
        Ok(text) => repo.append_assistant_message(&conversation.id, &text, "done"),
        Err(error_text) => repo.append_assistant_message(&conversation.id, &error_text, "error"),
    }
}

async fn run_snapshot_agent_turn(
    store: &LocalJsonStore,
    conversation: &AgentConversation,
    history: &[AgentConversationMessage],
) -> Result<String, String> {
    // Resolve provider settings
    let provider_repo = ProviderSettingRepository::new(store);
    let provider_settings = provider_repo.list()?;

    let provider_setting = provider_settings
        .iter()
        .find(|setting| setting.provider_id == conversation.snapshot_provider_id)
        .ok_or_else(|| {
            format!(
                "未找到 {} 的模型服务配置，请先在设置中配置",
                conversation.snapshot_provider_id
            )
        })?;

    if !provider_setting.enabled {
        return Err(format!("{} 已被禁用，请先启用后再对话", provider_setting.provider_id));
    }

    if provider_setting.api_key.trim().is_empty() {
        return Err(format!("{} 的 API Key 不能为空", provider_setting.provider_id));
    }

    if provider_setting.base_url.trim().is_empty() {
        return Err(format!("{} 的 Base URL 不能为空", provider_setting.provider_id));
    }

    // Build messages for rig
    let mut rig_messages = Vec::new();

    // Add system prompt from snapshot
    if !conversation.snapshot_system_prompt.trim().is_empty() {
        rig_messages.push(RigMessage::system(&conversation.snapshot_system_prompt));
    }

    // Add conversation history (excluding the newly appended user message)
    let user_msg_count = history.iter().filter(|m| m.role == "user").count();
    let mut current_user_count = 0;

    for msg in history {
        if msg.role == "user" {
            current_user_count += 1;
            // Skip the last user message as it will be the prompt
            if current_user_count == user_msg_count {
                continue;
            }
        }

        let rig_message = match msg.role.as_str() {
            "user" => RigMessage::user(&msg.content),
            "assistant" => RigMessage::assistant(&msg.content),
            "system" => RigMessage::system(&msg.content),
            _ => continue,
        };
        rig_messages.push(rig_message);
    }

    // Get the last user message as the prompt
    let last_user_message = history
        .iter()
        .filter(|m| m.role == "user")
        .last()
        .map(|m| m.content.as_str())
        .unwrap_or("");

    if last_user_message.is_empty() {
        return Err("用户消息不能为空".to_string());
    }

    // Create rig client
    let client = Client::builder()
        .api_key(provider_setting.api_key.trim())
        .base_url(provider_setting.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    // Build completion request
    let mut request = client
        .completion_model(&conversation.snapshot_model_id)
        .completion_request(RigMessage::user(last_user_message));

    // Add history messages (excluding the last user message)
    for msg in &rig_messages {
        request = request.message(msg.clone());
    }

    // Add optional parameters from snapshot
    if !conversation.snapshot_temperature.trim().is_empty() {
        if let Ok(temp) = conversation.snapshot_temperature.trim().parse::<f64>() {
            request = request.temperature(temp);
        }
    }

    if !conversation.snapshot_max_tokens.trim().is_empty() {
        if let Ok(max_tokens) = conversation.snapshot_max_tokens.trim().parse::<u64>() {
            request = request.max_tokens(max_tokens);
        }
    }

    // Send request
    let response = request
        .send()
        .await
        .map_err(|error| format!("模型请求失败: {error}"))?;

    // Extract response text
    let content = extract_text_response(response.choice)
        .ok_or_else(|| "模型未返回可展示的文本内容".to_string())?;

    Ok(content)
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

// Note: Integration tests for Tauri commands would require a test harness.
// The repository tests in agent_conversation.rs cover the core logic.

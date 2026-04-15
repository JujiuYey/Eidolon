use std::cell::RefCell;
use std::collections::HashMap;

use chrono::Utc;
use nanoid::nanoid;

use crate::db::local_store::LocalJsonStore;
use crate::models::agent_conversation::{AgentConversation, AgentConversationMessage};
use crate::models::agent_profile::AgentProfile;

const CONVERSATIONS_FILENAME: &str = "agent_conversations";
const MESSAGES_FILENAME: &str = "agent_conversation_messages";
const DEFAULT_TITLE: &str = "新对话";
const SNAPSHOT_VERSION: u32 = 1;

pub struct AgentConversationRepository<'a> {
    store: &'a LocalJsonStore,
    conversation_cache: RefCell<HashMap<String, AgentConversation>>,
    message_cache: RefCell<HashMap<String, Vec<AgentConversationMessage>>>,
}

impl<'a> AgentConversationRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let conversation_cache = store.read(CONVERSATIONS_FILENAME).unwrap_or_default();
        let message_cache = store.read(MESSAGES_FILENAME).unwrap_or_default();

        Self {
            store,
            conversation_cache: RefCell::new(conversation_cache),
            message_cache: RefCell::new(message_cache),
        }
    }

    pub fn list_by_agent(&self, agent_profile_id: &str) -> Result<Vec<AgentConversation>, String> {
        let cache = self.conversation_cache.borrow();
        let mut results = cache
            .values()
            .filter(|conversation| conversation.agent_profile_id == agent_profile_id)
            .cloned()
            .collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then(left.title.cmp(&right.title))
                .then(left.id.cmp(&right.id))
        });

        Ok(results)
    }

    pub fn list_recent(&self, limit: Option<usize>) -> Result<Vec<AgentConversation>, String> {
        let cache = self.conversation_cache.borrow();
        let mut results = cache.values().cloned().collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then(left.title.cmp(&right.title))
                .then(left.id.cmp(&right.id))
        });

        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    pub fn get(&self, conversation_id: &str) -> Result<Option<AgentConversation>, String> {
        Ok(self.conversation_cache.borrow().get(conversation_id).cloned())
    }

    pub fn create_from_profile(&self, profile: &AgentProfile) -> Result<AgentConversation, String> {
        let now = Utc::now().timestamp_millis();
        let conversation = AgentConversation {
            id: format!("conv_{}", nanoid!(10)),
            agent_profile_id: profile.id.clone(),
            title: DEFAULT_TITLE.to_string(),
            snapshot_version: SNAPSHOT_VERSION,
            created_from_profile_updated_at: profile.updated_at,
            snapshot_agent_name: profile.name.clone(),
            snapshot_provider_id: profile.provider_id.clone(),
            snapshot_model_id: profile.model_id.clone(),
            snapshot_temperature: profile.temperature.clone(),
            snapshot_max_tokens: profile.max_tokens.clone(),
            snapshot_system_prompt: profile.system_prompt.clone(),
            snapshot_enabled_mcp_service_ids: profile.enabled_mcp_service_ids.clone(),
            snapshot_enabled_tool_keys: profile.enabled_tool_keys.clone(),
            created_at: now,
            updated_at: now,
        };

        self.conversation_cache
            .borrow_mut()
            .insert(conversation.id.clone(), conversation.clone());
        self.message_cache
            .borrow_mut()
            .entry(conversation.id.clone())
            .or_default();

        self.persist()?;

        Ok(conversation)
    }

    pub fn delete(&self, conversation_id: &str) -> Result<String, String> {
        if self
            .conversation_cache
            .borrow_mut()
            .remove(conversation_id)
            .is_none()
        {
            return Err(format!("未找到 id 为 {} 的会话", conversation_id));
        }

        self.message_cache.borrow_mut().remove(conversation_id);
        self.persist()?;

        Ok(conversation_id.to_string())
    }

    pub fn list_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AgentConversationMessage>, String> {
        let mut messages = self
            .message_cache
            .borrow()
            .get(conversation_id)
            .cloned()
            .unwrap_or_default();
        messages.sort_by(|left, right| {
            left.created_at.cmp(&right.created_at).then(left.id.cmp(&right.id))
        });
        Ok(messages)
    }

    pub fn append_message(
        &self,
        message: &AgentConversationMessage,
    ) -> Result<AgentConversationMessage, String> {
        if !self
            .conversation_cache
            .borrow()
            .contains_key(message.conversation_id.as_str())
        {
            return Err(format!(
                "未找到 id 为 {} 的会话",
                message.conversation_id
            ));
        }

        let normalized = normalize_message(message)?;

        self.message_cache
            .borrow_mut()
            .entry(normalized.conversation_id.clone())
            .or_default()
            .push(normalized.clone());

        self.touch_conversation(&normalized.conversation_id, Some(&normalized.content))?;

        Ok(normalized)
    }

    pub fn append_user_message(
        &self,
        conversation_id: &str,
        content: &str,
    ) -> Result<AgentConversationMessage, String> {
        self.append_message(&AgentConversationMessage {
            conversation_id: conversation_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            status: "done".to_string(),
            ..Default::default()
        })
    }

    pub fn append_assistant_message(
        &self,
        conversation_id: &str,
        content: &str,
        status: &str,
    ) -> Result<AgentConversationMessage, String> {
        self.append_message(&AgentConversationMessage {
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.to_string(),
            status: status.to_string(),
            ..Default::default()
        })
    }

    fn touch_conversation(
        &self,
        conversation_id: &str,
        content_for_title: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().timestamp_millis();
        let mut conversations = self.conversation_cache.borrow_mut();
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| format!("未找到 id 为 {} 的会话", conversation_id))?;

        if conversation.title == DEFAULT_TITLE {
            if let Some(content) = content_for_title {
                if !content.trim().is_empty() {
                    conversation.title = build_conversation_title(content);
                }
            }
        }

        conversation.updated_at = now;
        drop(conversations);
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        self.store
            .write(CONVERSATIONS_FILENAME, &*self.conversation_cache.borrow())?;
        self.store
            .write(MESSAGES_FILENAME, &*self.message_cache.borrow())?;
        Ok(())
    }
}

fn normalize_message(message: &AgentConversationMessage) -> Result<AgentConversationMessage, String> {
    let conversation_id = message.conversation_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversation_id 不能为空".to_string());
    }

    let role = message.role.trim().to_string();
    if !matches!(role.as_str(), "user" | "assistant" | "system") {
        return Err(format!("不支持的消息角色: {}", role));
    }

    let content = message.content.trim().to_string();
    if content.is_empty() {
        return Err("消息内容不能为空".to_string());
    }

    let now = Utc::now().timestamp_millis();

    Ok(AgentConversationMessage {
        id: if message.id.trim().is_empty() {
            format!("msg_{}", nanoid!(10))
        } else {
            message.id.trim().to_string()
        },
        conversation_id,
        role,
        content,
        status: if message.status.trim().is_empty() {
            "done".to_string()
        } else {
            message.status.trim().to_string()
        },
        created_at: if message.created_at > 0 {
            message.created_at
        } else {
            now
        },
    })
}

fn build_conversation_title(content: &str) -> String {
    let normalized = content.replace(char::is_whitespace, " ");
    let trimmed = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return DEFAULT_TITLE.to_string();
    }

    let mut chars = trimmed.chars();
    let title = chars.by_ref().take(18).collect::<String>();
    if chars.next().is_some() {
        format!("{title}...")
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::AgentConversationRepository;
    use crate::db::local_store::LocalJsonStore;
    use crate::db::repositories::agent_profile::AgentProfileRepository;
    use crate::models::agent_profile::AgentProfile;

    fn create_profile_repo_and_store() -> (tempfile::TempDir, LocalJsonStore) {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        (temp_dir, store)
    }

    fn persist_profile(store: &LocalJsonStore, name: &str, updated_at: i64) -> AgentProfile {
        let profile_repo = AgentProfileRepository::new(store);
        let profile_id = profile_repo
            .upsert(&AgentProfile {
                name: name.to_string(),
                provider_id: "openai".to_string(),
                model_id: "gpt-4.1".to_string(),
                temperature: "0.3".to_string(),
                max_tokens: "2048".to_string(),
                system_prompt: format!("System prompt for {name}"),
                enabled_mcp_service_ids: vec!["mcp_filesystem".to_string()],
                enabled_tool_keys: vec!["mcp_filesystem:read_file".to_string()],
                updated_at,
                ..Default::default()
            })
            .expect("profile should persist");

        profile_repo
            .get(&profile_id)
            .expect("lookup should succeed")
            .expect("profile should exist")
    }

    #[test]
    fn create_conversation_snapshots_agent_profile() {
        let (_temp_dir, store) = create_profile_repo_and_store();
        let conversation_repo = AgentConversationRepository::new(&store);
        let profile = persist_profile(&store, "Filesystem Assistant", 42);

        let conversation = conversation_repo
            .create_from_profile(&profile)
            .expect("conversation should be created");

        assert_eq!(conversation.agent_profile_id, profile.id);
        assert_eq!(conversation.snapshot_model_id, "gpt-4.1");
        assert_eq!(
            conversation.snapshot_enabled_tool_keys,
            vec!["mcp_filesystem:read_file".to_string()]
        );
        assert_eq!(conversation.created_from_profile_updated_at, 42);
    }

    #[test]
    fn list_by_agent_sorts_by_updated_at_descending() {
        let (_temp_dir, store) = create_profile_repo_and_store();
        let conversation_repo = AgentConversationRepository::new(&store);
        let profile = persist_profile(&store, "Filesystem Assistant", 1);

        let older = conversation_repo
            .create_from_profile(&profile)
            .expect("older conversation should be created");

        std::thread::sleep(std::time::Duration::from_millis(1));

        let newer = conversation_repo
            .create_from_profile(&profile)
            .expect("newer conversation should be created");

        let conversations = conversation_repo
            .list_by_agent(&profile.id)
            .expect("conversation list should load");

        assert_eq!(conversations[0].id, newer.id);
        assert_eq!(conversations[1].id, older.id);
    }

    #[test]
    fn append_message_updates_conversation_timestamp_and_message_order() {
        let (_temp_dir, store) = create_profile_repo_and_store();
        let conversation_repo = AgentConversationRepository::new(&store);
        let profile = persist_profile(&store, "Filesystem Assistant", 1);
        let conversation = conversation_repo
            .create_from_profile(&profile)
            .expect("conversation should be created");
        let original_updated_at = conversation.updated_at;

        let first = conversation_repo
            .append_user_message(&conversation.id, "First message")
            .expect("first message should append");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let second = conversation_repo
            .append_assistant_message(&conversation.id, "Second message", "done")
            .expect("second message should append");

        let reloaded = conversation_repo
            .get(&conversation.id)
            .expect("conversation lookup should succeed")
            .expect("conversation should exist");
        assert!(reloaded.updated_at >= original_updated_at);

        let messages = conversation_repo
            .list_messages(&conversation.id)
            .expect("messages should load");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, first.id);
        assert_eq!(messages[1].id, second.id);
        assert_eq!(reloaded.title, "First message");
    }

    #[test]
    fn delete_removes_conversation_and_messages() {
        let (_temp_dir, store) = create_profile_repo_and_store();
        let conversation_repo = AgentConversationRepository::new(&store);
        let profile = persist_profile(&store, "Filesystem Assistant", 1);
        let conversation = conversation_repo
            .create_from_profile(&profile)
            .expect("conversation should be created");

        conversation_repo
            .append_user_message(&conversation.id, "Delete me")
            .expect("message should append");

        conversation_repo
            .delete(&conversation.id)
            .expect("conversation should delete");

        assert!(
            conversation_repo
                .get(&conversation.id)
                .expect("lookup should succeed")
                .is_none()
        );
        assert!(
            conversation_repo
                .list_messages(&conversation.id)
                .expect("messages should load")
                .is_empty()
        );
    }

    #[test]
    fn list_recent_returns_conversations_across_agents_descending() {
        let (_temp_dir, store) = create_profile_repo_and_store();
        let conversation_repo = AgentConversationRepository::new(&store);
        let first_profile = persist_profile(&store, "Filesystem Assistant", 1);
        let second_profile = persist_profile(&store, "Docs Assistant", 2);

        let older = conversation_repo
            .create_from_profile(&first_profile)
            .expect("older conversation should be created");

        std::thread::sleep(std::time::Duration::from_millis(1));

        let newer = conversation_repo
            .create_from_profile(&second_profile)
            .expect("newer conversation should be created");

        let recent = conversation_repo
            .list_recent(Some(10))
            .expect("recent conversations should load");

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, newer.id);
        assert_eq!(recent[1].id, older.id);
    }
}

use std::cell::RefCell;
use std::collections::HashMap;

use chrono::Utc;
use nanoid::nanoid;

use crate::db::local_store::LocalJsonStore;
use crate::models::pm::{PmConversation, PmMessage};

const CONVERSATIONS_FILENAME: &str = "pm_conversations";
const MESSAGES_FILENAME: &str = "pm_conversation_messages";
const DEFAULT_TITLE: &str = "新对话";
const MAX_TITLE_CHARS: usize = 50;

pub struct PmConversationRepository<'a> {
    store: &'a LocalJsonStore,
    conversation_cache: RefCell<HashMap<String, PmConversation>>,
    message_cache: RefCell<HashMap<String, Vec<PmMessage>>>,
}

impl<'a> PmConversationRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let conversation_cache = store.read(CONVERSATIONS_FILENAME).unwrap_or_default();
        let message_cache = store.read(MESSAGES_FILENAME).unwrap_or_default();

        Self {
            store,
            conversation_cache: RefCell::new(conversation_cache),
            message_cache: RefCell::new(message_cache),
        }
    }

    pub fn list(&self) -> Result<Vec<PmConversation>, String> {
        let cache = self.conversation_cache.borrow();
        let mut results = cache.values().cloned().collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then(left.title.cmp(&right.title))
                .then(left.id.cmp(&right.id))
        });

        Ok(results)
    }

    pub fn get(&self, conversation_id: &str) -> Result<Option<PmConversation>, String> {
        Ok(self
            .conversation_cache
            .borrow()
            .get(conversation_id)
            .cloned())
    }

    pub fn create(&self) -> Result<PmConversation, String> {
        let now = Utc::now().timestamp_millis();
        let conversation = PmConversation {
            id: format!("conv_{}", nanoid!(10)),
            title: DEFAULT_TITLE.to_string(),
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

    pub fn rename(&self, conversation_id: &str, title: &str) -> Result<PmConversation, String> {
        let normalized = normalize_title(title)?;

        let mut conversations = self.conversation_cache.borrow_mut();
        let conversation = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| format!("未找到 id 为 {} 的会话", conversation_id))?;

        conversation.title = normalized;
        conversation.updated_at = Utc::now().timestamp_millis();
        let updated = conversation.clone();
        drop(conversations);

        self.persist()?;

        Ok(updated)
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

    pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<PmMessage>, String> {
        let mut messages = self
            .message_cache
            .borrow()
            .get(conversation_id)
            .cloned()
            .unwrap_or_default();
        messages.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        Ok(messages)
    }

    pub fn append_user_message(
        &self,
        conversation_id: &str,
        content: &str,
    ) -> Result<PmMessage, String> {
        self.append_message(PmMessage {
            conversation_id: conversation_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            ..Default::default()
        })
    }

    pub fn append_assistant_message(
        &self,
        conversation_id: &str,
        content: &str,
        skills_used: Vec<String>,
    ) -> Result<PmMessage, String> {
        self.append_message(PmMessage {
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.to_string(),
            skills_used,
            ..Default::default()
        })
    }

    fn append_message(&self, message: PmMessage) -> Result<PmMessage, String> {
        if !self
            .conversation_cache
            .borrow()
            .contains_key(message.conversation_id.as_str())
        {
            return Err(format!("未找到 id 为 {} 的会话", message.conversation_id));
        }

        let normalized = normalize_message(message)?;

        self.message_cache
            .borrow_mut()
            .entry(normalized.conversation_id.clone())
            .or_default()
            .push(normalized.clone());

        // 只有用户消息参与自动命名，避免助手首句变成标题
        let title_content = if normalized.role == "user" {
            Some(normalized.content.as_str())
        } else {
            None
        };
        self.touch_conversation(&normalized.conversation_id, title_content)?;

        Ok(normalized)
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

fn normalize_message(message: PmMessage) -> Result<PmMessage, String> {
    let conversation_id = message.conversation_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversation_id 不能为空".to_string());
    }

    let role = message.role.trim().to_string();
    if !matches!(role.as_str(), "user" | "assistant") {
        return Err(format!("不支持的消息角色: {}", role));
    }

    let content = message.content.trim().to_string();
    if content.is_empty() {
        return Err("消息内容不能为空".to_string());
    }

    Ok(PmMessage {
        id: if message.id.trim().is_empty() {
            format!("msg_{}", nanoid!(10))
        } else {
            message.id.trim().to_string()
        },
        conversation_id,
        role,
        content,
        skills_used: message.skills_used,
        created_at: if message.created_at > 0 {
            message.created_at
        } else {
            Utc::now().timestamp_millis()
        },
    })
}

fn normalize_title(title: &str) -> Result<String, String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("会话标题不能为空".to_string());
    }

    let mut chars = trimmed.chars();
    let title = chars.by_ref().take(MAX_TITLE_CHARS).collect::<String>();
    if chars.next().is_some() {
        Ok(format!("{title}..."))
    } else {
        Ok(title)
    }
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

    use super::PmConversationRepository;
    use crate::db::local_store::LocalJsonStore;

    fn create_store() -> (tempfile::TempDir, LocalJsonStore) {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        (temp_dir, store)
    }

    #[test]
    fn create_returns_conversation_with_defaults() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);

        let conversation = repo.create().expect("conversation should be created");

        assert!(conversation.id.starts_with("conv_"));
        assert_eq!(conversation.title, "新对话");
        assert!(conversation.created_at > 0);
        assert_eq!(conversation.created_at, conversation.updated_at);
    }

    #[test]
    fn list_sorts_by_updated_at_descending() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);

        let older = repo.create().expect("older conversation should be created");

        std::thread::sleep(std::time::Duration::from_millis(1));

        let newer = repo.create().expect("newer conversation should be created");

        let conversations = repo.list().expect("list should load");
        assert_eq!(conversations[0].id, newer.id);
        assert_eq!(conversations[1].id, older.id);
    }

    #[test]
    fn append_messages_sets_title_from_user_message_and_keeps_order() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);
        let conversation = repo.create().expect("conversation should be created");

        let user = repo
            .append_user_message(&conversation.id, "帮我看下这个登录流程")
            .expect("user message should append");

        std::thread::sleep(std::time::Duration::from_millis(1));

        let assistant = repo
            .append_assistant_message(
                &conversation.id,
                "好的，我们从任务/Bug 列表开始梳理",
                vec!["ux-heuristics-review".to_string()],
            )
            .expect("assistant message should append");

        let messages = repo.list_messages(&conversation.id).expect("messages load");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, user.id);
        assert_eq!(messages[1].id, assistant.id);
        assert_eq!(
            messages[1].skills_used,
            vec!["ux-heuristics-review".to_string()]
        );

        let reloaded = repo
            .get(&conversation.id)
            .expect("lookup should succeed")
            .expect("conversation should exist");
        assert_eq!(reloaded.title, "帮我看下这个登录流程");
    }

    #[test]
    fn rename_updates_title_and_persists() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);
        let conversation = repo.create().expect("conversation should be created");

        let renamed = repo
            .rename(&conversation.id, "  登录流程评审  ")
            .expect("rename should succeed");
        assert_eq!(renamed.title, "登录流程评审");

        // 重新实例化仓储验证落盘
        let reloaded_repo = PmConversationRepository::new(&store);
        let reloaded = reloaded_repo
            .get(&conversation.id)
            .expect("lookup should succeed")
            .expect("conversation should exist");
        assert_eq!(reloaded.title, "登录流程评审");
    }

    #[test]
    fn rename_rejects_empty_title_and_missing_conversation() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);
        let conversation = repo.create().expect("conversation should be created");

        assert!(repo.rename(&conversation.id, "   ").is_err());
        assert!(repo.rename("conv_missing", "标题").is_err());
    }

    #[test]
    fn delete_removes_conversation_and_messages() {
        let (_temp_dir, store) = create_store();
        let repo = PmConversationRepository::new(&store);
        let conversation = repo.create().expect("conversation should be created");

        repo.append_user_message(&conversation.id, "Delete me")
            .expect("message should append");

        repo.delete(&conversation.id).expect("delete should succeed");

        assert!(repo
            .get(&conversation.id)
            .expect("lookup should succeed")
            .is_none());
        assert!(repo
            .list_messages(&conversation.id)
            .expect("messages should load")
            .is_empty());
    }
}

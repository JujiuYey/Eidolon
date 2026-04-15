use std::cell::RefCell;
use std::collections::HashMap;

use chrono::Utc;
use nanoid::nanoid;

use crate::db::local_store::LocalJsonStore;
use crate::models::agent_profile::AgentProfile;

const PROFILES_FILENAME: &str = "agent_profiles";

pub struct AgentProfileRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, AgentProfile>>,
}

impl<'a> AgentProfileRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let cache = store.read(PROFILES_FILENAME).unwrap_or_default();

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<AgentProfile>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then(left.id.cmp(&right.id))
        });
        Ok(results)
    }

    pub fn get(&self, profile_id: &str) -> Result<Option<AgentProfile>, String> {
        Ok(self.cache.borrow().get(profile_id).cloned())
    }

    pub fn upsert(&self, profile: &AgentProfile) -> Result<String, String> {
        let existing = if profile.id.trim().is_empty() {
            None
        } else {
            self.cache.borrow().get(profile.id.trim()).cloned()
        };

        let normalized = normalize_profile(profile, existing.as_ref())?;
        let profile_id = normalized.id.clone();

        self.cache.borrow_mut().insert(profile_id.clone(), normalized);
        self.store.write(PROFILES_FILENAME, &*self.cache.borrow())?;

        Ok(profile_id)
    }

    pub fn delete(&self, profile_id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(profile_id).is_none() {
            return Err(format!("未找到 id 为 {} 的 Agent", profile_id));
        }

        self.store.write(PROFILES_FILENAME, &*self.cache.borrow())?;
        Ok(profile_id.to_string())
    }
}

fn normalize_profile(
    profile: &AgentProfile,
    existing: Option<&AgentProfile>,
) -> Result<AgentProfile, String> {
    let name = profile.name.trim().to_string();
    if name.is_empty() {
        return Err("Agent 名称不能为空".to_string());
    }

    let provider_id = profile.provider_id.trim().to_string();
    if provider_id.is_empty() {
        return Err("provider_id 不能为空".to_string());
    }

    let model_id = profile.model_id.trim().to_string();
    if model_id.is_empty() {
        return Err("model_id 不能为空".to_string());
    }

    let system_prompt = profile.system_prompt.trim().to_string();
    if system_prompt.is_empty() {
        return Err("system_prompt 不能为空".to_string());
    }

    let work_directory = profile.work_directory.trim().to_string();

    let now = Utc::now().timestamp_millis();
    let id = if profile.id.trim().is_empty() {
        format!("agent_{}", nanoid!(10))
    } else {
        profile.id.trim().to_string()
    };

    let created_at = existing
        .map(|item| item.created_at)
        .or_else(|| (profile.created_at > 0).then_some(profile.created_at))
        .unwrap_or(now);

    let updated_at = if profile.updated_at > 0 {
        profile.updated_at
    } else {
        now
    };

    Ok(AgentProfile {
        id,
        name,
        description: profile.description.trim().to_string(),
        provider_id,
        model_id,
        temperature: normalize_optional_string(&profile.temperature, "0.7"),
        max_tokens: normalize_optional_string(&profile.max_tokens, "4096"),
        system_prompt,
        work_directory,
        enabled_mcp_service_ids: normalize_list(&profile.enabled_mcp_service_ids),
        enabled_tool_keys: normalize_list(&profile.enabled_tool_keys),
        created_at,
        updated_at,
    })
}

fn normalize_optional_string(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;

    use tempfile::tempdir;

    use super::AgentProfileRepository;
    use crate::db::local_store::LocalJsonStore;
    use crate::models::agent_profile::AgentProfile;

    #[test]
    fn upsert_generates_agent_id_and_persists_profile() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = AgentProfileRepository::new(&store);

        let profile = AgentProfile {
            name: "Filesystem Assistant".to_string(),
            description: "Manage local files safely.".to_string(),
            provider_id: "openai".to_string(),
            model_id: "gpt-4.1".to_string(),
            system_prompt: "Help me manage local files safely.".to_string(),
            work_directory: "/tmp/eidolon-workspace".to_string(),
            enabled_mcp_service_ids: vec!["mcp_filesystem".to_string()],
            enabled_tool_keys: vec!["mcp_filesystem:read_file".to_string()],
            ..Default::default()
        };

        let profile_id = repository
            .upsert(&profile)
            .expect("profile should be persisted");

        assert!(profile_id.starts_with("agent_"));

        let persisted_path = temp_dir.path().join("agent_profiles.json");
        assert!(persisted_path.exists());

        let content =
            fs::read_to_string(&persisted_path).expect("persisted profiles should be readable");
        let persisted: HashMap<String, AgentProfile> =
            serde_json::from_str(&content).expect("profiles should deserialize");
        let stored = persisted
            .get(&profile_id)
            .expect("persisted profile should exist");

        assert_eq!(stored.enabled_mcp_service_ids, vec!["mcp_filesystem"]);
        assert_eq!(stored.enabled_tool_keys, vec!["mcp_filesystem:read_file"]);
        assert_eq!(stored.work_directory, "/tmp/eidolon-workspace");
    }

    #[test]
    fn upsert_normalizes_blank_work_directory() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = AgentProfileRepository::new(&store);

        let profile_id = repository
            .upsert(&AgentProfile {
                name: "Chat Only".to_string(),
                provider_id: "openai".to_string(),
                model_id: "gpt-4.1".to_string(),
                system_prompt: "Chat safely.".to_string(),
                work_directory: "   ".to_string(),
                ..Default::default()
            })
            .expect("profile should persist");

        let stored = repository
            .get(&profile_id)
            .expect("lookup should succeed")
            .expect("profile should exist");

        assert_eq!(stored.work_directory, "");
    }

    #[test]
    fn list_sorts_by_updated_at_descending() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = AgentProfileRepository::new(&store);

        let older_id = repository
            .upsert(&AgentProfile {
                name: "Older".to_string(),
                provider_id: "openai".to_string(),
                model_id: "gpt-4.1".to_string(),
                system_prompt: "Older profile".to_string(),
                updated_at: 1,
                ..Default::default()
            })
            .expect("older profile should persist");

        let newer_id = repository
            .upsert(&AgentProfile {
                name: "Newer".to_string(),
                provider_id: "openai".to_string(),
                model_id: "gpt-4.1".to_string(),
                system_prompt: "Newer profile".to_string(),
                updated_at: 2,
                ..Default::default()
            })
            .expect("newer profile should persist");

        let profiles = repository.list().expect("profiles should list");

        assert_eq!(profiles[0].id, newer_id);
        assert_eq!(profiles[1].id, older_id);
    }

    #[test]
    fn delete_removes_persisted_profile() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = AgentProfileRepository::new(&store);

        let profile_id = repository
            .upsert(&AgentProfile {
                name: "Delete Me".to_string(),
                provider_id: "openai".to_string(),
                model_id: "gpt-4.1".to_string(),
                system_prompt: "Delete profile".to_string(),
                ..Default::default()
            })
            .expect("profile should be persisted");

        repository
            .delete(&profile_id)
            .expect("profile should be deleted");

        assert!(
            repository
                .get(&profile_id)
                .expect("profile lookup should succeed")
                .is_none()
        );
    }
}

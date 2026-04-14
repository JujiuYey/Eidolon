use std::cell::RefCell;
use std::collections::HashMap;

use serde::Deserialize;

use crate::db::local_store::LocalJsonStore;
use crate::models::model_config::ProviderConfig;

const FILENAME: &str = "provider_configs";
const LEGACY_FILENAME: &str = "model_provider_settings";

pub struct ProviderConfigRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, ProviderConfig>>,
}

impl<'a> ProviderConfigRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let (cache, migrated) = load_cache(store).unwrap_or_else(|_| (HashMap::default(), false));

        if migrated {
            let _ = store.write(FILENAME, &cache);
        }

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<ProviderConfig>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        Ok(results)
    }

    pub fn upsert(&self, config: &ProviderConfig) -> Result<String, String> {
        if config.provider_id.trim().is_empty() {
            return Err("provider_id 不能为空".to_string());
        }

        let key = config.provider_id.clone();
        self.cache.borrow_mut().insert(key.clone(), config.clone());
        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(key)
    }

    pub fn delete(&self, provider_id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(provider_id).is_none() {
            return Err(format!("未找到 provider_id 为 {} 的配置", provider_id));
        }

        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(provider_id.to_string())
    }
}

fn load_cache(store: &LocalJsonStore) -> Result<(HashMap<String, ProviderConfig>, bool), String> {
    let cache: HashMap<String, ProviderConfig> = store.read(FILENAME)?;
    if !cache.is_empty() {
        return Ok((cache, false));
    }

    let legacy_cache: HashMap<String, LegacyProviderConfig> = store.read(LEGACY_FILENAME)?;
    if legacy_cache.is_empty() {
        return Ok((cache, false));
    }

    let migrated = legacy_cache
        .into_iter()
        .map(|(legacy_key, legacy)| {
            let provider_id = if legacy.provider_key.trim().is_empty() {
                legacy_key
            } else {
                legacy.provider_key
            };

            let config = ProviderConfig {
                provider_id: provider_id.clone(),
                enabled: legacy.enabled,
                api_key: legacy.connection.api_key.unwrap_or_default(),
                base_url: legacy.connection.base_url.unwrap_or_default(),
            };

            (provider_id, config)
        })
        .collect();

    Ok((migrated, true))
}

#[derive(Debug, Deserialize)]
struct LegacyProviderConfig {
    #[serde(rename = "_id", default)]
    provider_key: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    connection: LegacyProviderConnection,
}

#[derive(Debug, Default, Deserialize)]
struct LegacyProviderConnection {
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::ProviderConfigRepository;
    use crate::db::local_store::LocalJsonStore;
    use crate::models::model_config::ProviderConfig;

    #[test]
    fn upsert_persists_provider_configs_without_runtime_model_fields() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = ProviderConfigRepository::new(&store);

        let config = ProviderConfig {
            provider_id: "deepseek".to_string(),
            enabled: true,
            api_key: "sk-test".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
        };

        repository
            .upsert(&config)
            .expect("config should be persisted");

        let persisted_path = temp_dir.path().join("provider_configs.json");
        assert!(
            persisted_path.exists(),
            "provider configs should use the new filename"
        );

        let content = fs::read_to_string(&persisted_path)
            .expect("persisted provider config should be readable");
        assert!(
            !content.contains("\"selected_model_id\""),
            "runtime selected model state should not be persisted",
        );
        assert!(
            !content.contains("\"catalog\""),
            "runtime model catalog should not be persisted",
        );
        assert!(
            !content.contains("\"created_at\""),
            "provider config should not persist created_at metadata",
        );
        assert!(
            !content.contains("\"updated_at\""),
            "provider config should not persist updated_at metadata",
        );
    }

    #[test]
    fn new_migrates_legacy_provider_config_shape() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let legacy_path = temp_dir.path().join("model_provider_settings.json");

        fs::write(
            &legacy_path,
            r#"{
  "deepseek": {
    "_id": "deepseek",
    "enabled": true,
    "connection": {
      "api_key": "sk-legacy",
      "base_url": "https://api.deepseek.com/v1"
    },
    "selected_model_id": "deepseek-chat",
    "catalog": {
      "items": [
        { "id": "deepseek-chat", "enabled": true }
      ]
    }
  }
}"#,
        )
        .expect("legacy fixture should be written");

        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");

        let repository = ProviderConfigRepository::new(&store);
        let configs = repository.list().expect("legacy configs should migrate");

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].provider_id, "deepseek");
        assert_eq!(configs[0].api_key, "sk-legacy");
        assert_eq!(configs[0].base_url, "https://api.deepseek.com/v1");

        let migrated_path = temp_dir.path().join("provider_configs.json");
        assert!(
            migrated_path.exists(),
            "legacy data should be migrated forward"
        );

        let migrated =
            fs::read_to_string(migrated_path).expect("migrated provider config should be readable");
        assert!(
            !migrated.contains("\"selected_model_id\""),
            "migrated config should drop runtime model state",
        );
        assert!(
            !migrated.contains("\"catalog\""),
            "migrated config should drop runtime catalog state",
        );
        assert!(
            !migrated.contains("\"created_at\""),
            "migrated config should drop created_at metadata",
        );
        assert!(
            !migrated.contains("\"updated_at\""),
            "migrated config should drop updated_at metadata",
        );
    }
}

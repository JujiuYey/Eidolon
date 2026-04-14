use std::cell::RefCell;
use std::collections::HashMap;

use serde::Deserialize;

use crate::db::local_store::LocalJsonStore;
use crate::models::model_config::{
    ProviderModel, ProviderModelCapabilities, ProviderSetting,
};

const SETTINGS_FILENAME: &str = "provider_settings";
const MODELS_FILENAME: &str = "provider_models";
const FLAT_LEGACY_FILENAME: &str = "provider_configs";
const MONOLITH_LEGACY_FILENAME: &str = "model_provider_settings";

pub struct ProviderSettingRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, ProviderSetting>>,
}

impl<'a> ProviderSettingRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let (cache, migrated) =
            load_provider_setting_cache(store).unwrap_or_else(|_| (HashMap::default(), false));

        if migrated {
            let _ = store.write(SETTINGS_FILENAME, &cache);
        }

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<ProviderSetting>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        Ok(results)
    }

    pub fn upsert(&self, setting: &ProviderSetting) -> Result<String, String> {
        if setting.provider_id.trim().is_empty() {
            return Err("provider_id 不能为空".to_string());
        }

        let key = setting.provider_id.clone();
        self.cache.borrow_mut().insert(key.clone(), setting.clone());
        self.store.write(SETTINGS_FILENAME, &*self.cache.borrow())?;

        Ok(key)
    }

    pub fn delete(&self, provider_id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(provider_id).is_none() {
            return Err(format!("未找到 provider_id 为 {} 的配置", provider_id));
        }

        self.store.write(SETTINGS_FILENAME, &*self.cache.borrow())?;
        Ok(provider_id.to_string())
    }
}

pub struct ProviderModelRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<Vec<ProviderModel>>,
}

impl<'a> ProviderModelRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let (cache, migrated) =
            load_provider_model_cache(store).unwrap_or_else(|_| (Vec::new(), false));

        if migrated {
            let _ = store.write(MODELS_FILENAME, &cache);
        }

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<ProviderModel>, String> {
        let mut results = self.cache.borrow().clone();
        sort_models(&mut results);
        Ok(results)
    }

    pub fn replace_for_provider(
        &self,
        provider_id: &str,
        models: &[ProviderModel],
    ) -> Result<String, String> {
        if provider_id.trim().is_empty() {
            return Err("provider_id 不能为空".to_string());
        }

        for model in models {
            if model.provider_id.trim().is_empty() || model.provider_id != provider_id {
                return Err("模型 provider_id 不合法".to_string());
            }

            if model.model_id.trim().is_empty() {
                return Err("模型 model_id 不能为空".to_string());
            }
        }

        let mut cache = self.cache.borrow_mut();
        cache.retain(|model| model.provider_id != provider_id);
        cache.extend(models.iter().cloned());
        sort_models(&mut cache);

        self.store.write(MODELS_FILENAME, &*cache)?;
        Ok(provider_id.to_string())
    }

    pub fn delete_by_provider(&self, provider_id: &str) -> Result<String, String> {
        self.cache.borrow_mut().retain(|model| model.provider_id != provider_id);
        self.store.write(MODELS_FILENAME, &*self.cache.borrow())?;
        Ok(provider_id.to_string())
    }
}

fn sort_models(models: &mut [ProviderModel]) {
    models.sort_by(|a, b| {
        a.provider_id
            .cmp(&b.provider_id)
            .then(a.model_id.cmp(&b.model_id))
    });
}

fn load_provider_setting_cache(
    store: &LocalJsonStore,
) -> Result<(HashMap<String, ProviderSetting>, bool), String> {
    let cache: HashMap<String, ProviderSetting> = store.read(SETTINGS_FILENAME)?;
    if !cache.is_empty() {
        return Ok((cache, false));
    }

    let flat_legacy: HashMap<String, FlatLegacyProviderConfig> = store.read(FLAT_LEGACY_FILENAME)?;
    if !flat_legacy.is_empty() {
        let migrated = flat_legacy
            .into_iter()
            .map(|(legacy_key, legacy)| {
                let provider_id = if legacy.provider_id.trim().is_empty() {
                    legacy_key
                } else {
                    legacy.provider_id
                };

                let setting = ProviderSetting {
                    provider_id: provider_id.clone(),
                    enabled: legacy.enabled,
                    api_key: legacy.api_key,
                    base_url: legacy.base_url,
                };

                (provider_id, setting)
            })
            .collect();

        return Ok((migrated, true));
    }

    let monolith_legacy: HashMap<String, MonolithLegacyProviderConfig> =
        store.read(MONOLITH_LEGACY_FILENAME)?;
    if monolith_legacy.is_empty() {
        return Ok((cache, false));
    }

    let migrated = monolith_legacy
        .into_iter()
        .map(|(legacy_key, legacy)| {
            let provider_id = if legacy.provider_key.trim().is_empty() {
                legacy_key
            } else {
                legacy.provider_key
            };

            let setting = ProviderSetting {
                provider_id: provider_id.clone(),
                enabled: legacy.enabled,
                api_key: legacy.connection.api_key.unwrap_or_default(),
                base_url: legacy.connection.base_url.unwrap_or_default(),
            };

            (provider_id, setting)
        })
        .collect();

    Ok((migrated, true))
}

fn load_provider_model_cache(
    store: &LocalJsonStore,
) -> Result<(Vec<ProviderModel>, bool), String> {
    let cache: Vec<ProviderModel> = store.read(MODELS_FILENAME)?;
    if !cache.is_empty() {
        return Ok((cache, false));
    }

    let flat_legacy: HashMap<String, FlatLegacyProviderConfig> = store.read(FLAT_LEGACY_FILENAME)?;
    if !flat_legacy.is_empty() {
        let mut migrated = Vec::new();

        for (legacy_key, legacy) in flat_legacy {
            let provider_id = if legacy.provider_id.trim().is_empty() {
                legacy_key
            } else {
                legacy.provider_id
            };

            for model in legacy.models {
                if model.model_id.trim().is_empty() {
                    continue;
                }

                migrated.push(ProviderModel {
                    provider_id: provider_id.clone(),
                    model_id: model.model_id,
                    capabilities: model.capabilities.into(),
                });
            }
        }

        sort_models(&mut migrated);
        return Ok((migrated, true));
    }

    let monolith_legacy: HashMap<String, MonolithLegacyProviderConfig> =
        store.read(MONOLITH_LEGACY_FILENAME)?;
    if monolith_legacy.is_empty() {
        return Ok((cache, false));
    }

    let mut migrated = Vec::new();

    for (legacy_key, legacy) in monolith_legacy {
        let provider_id = if legacy.provider_key.trim().is_empty() {
            legacy_key
        } else {
            legacy.provider_key
        };

        for model in legacy.catalog.items {
            if model.id.trim().is_empty() {
                continue;
            }

            migrated.push(ProviderModel {
                provider_id: provider_id.clone(),
                model_id: model.id,
                capabilities: model.capabilities.into(),
            });
        }
    }

    sort_models(&mut migrated);
    Ok((migrated, true))
}

#[derive(Debug, Deserialize)]
struct FlatLegacyProviderConfig {
    #[serde(default)]
    provider_id: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    models: Vec<FlatLegacyProviderModel>,
}

#[derive(Debug, Default, Deserialize)]
struct FlatLegacyProviderModel {
    #[serde(default, alias = "id")]
    model_id: String,
    #[serde(default)]
    capabilities: LegacyProviderModelCapabilities,
}

#[derive(Debug, Deserialize)]
struct MonolithLegacyProviderConfig {
    #[serde(rename = "_id", default)]
    provider_key: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    connection: MonolithLegacyProviderConnection,
    #[serde(default)]
    catalog: MonolithLegacyProviderCatalog,
}

#[derive(Debug, Default, Deserialize)]
struct MonolithLegacyProviderConnection {
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct MonolithLegacyProviderCatalog {
    #[serde(default)]
    items: Vec<MonolithLegacyProviderModel>,
}

#[derive(Debug, Default, Deserialize)]
struct MonolithLegacyProviderModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    capabilities: LegacyProviderModelCapabilities,
}

#[derive(Debug, Default, Deserialize)]
struct LegacyProviderModelCapabilities {
    #[serde(default)]
    chat: bool,
    #[serde(default)]
    vision: bool,
    #[serde(default)]
    tool_call: bool,
    #[serde(default)]
    reasoning: bool,
    #[serde(default)]
    embedding: bool,
}

impl From<LegacyProviderModelCapabilities> for ProviderModelCapabilities {
    fn from(value: LegacyProviderModelCapabilities) -> Self {
        Self {
            chat: value.chat,
            vision: value.vision,
            tool_call: value.tool_call,
            reasoning: value.reasoning,
            embedding: value.embedding,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ProviderModelRepository, ProviderSettingRepository};
    use crate::db::local_store::LocalJsonStore;
    use crate::models::model_config::{
        ProviderModel, ProviderModelCapabilities, ProviderSetting,
    };

    #[test]
    fn upsert_persists_provider_settings_without_model_fields() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = ProviderSettingRepository::new(&store);

        let setting = ProviderSetting {
            provider_id: "deepseek".to_string(),
            enabled: true,
            api_key: "sk-test".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
        };

        repository
            .upsert(&setting)
            .expect("setting should be persisted");

        let persisted_path = temp_dir.path().join("provider_settings.json");
        assert!(persisted_path.exists());

        let content = fs::read_to_string(&persisted_path)
            .expect("persisted provider setting should be readable");
        assert!(!content.contains("\"models\""));
    }

    #[test]
    fn replace_for_provider_persists_models_separately() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = ProviderModelRepository::new(&store);

        repository
            .replace_for_provider(
                "deepseek",
                &[ProviderModel {
                    provider_id: "deepseek".to_string(),
                    model_id: "deepseek-chat".to_string(),
                    capabilities: ProviderModelCapabilities {
                        chat: true,
                        vision: false,
                        tool_call: true,
                        reasoning: false,
                        embedding: false,
                    },
                }],
            )
            .expect("models should be persisted");

        let persisted_path = temp_dir.path().join("provider_models.json");
        assert!(persisted_path.exists());

        let content = fs::read_to_string(&persisted_path)
            .expect("persisted provider models should be readable");
        assert!(content.contains("\"provider_id\""));
        assert!(content.contains("\"model_id\""));
        assert!(content.contains("\"capabilities\""));
    }

    #[test]
    fn repositories_migrate_legacy_monolith_shape() {
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
    "catalog": {
      "items": [
        {
          "id": "deepseek-chat",
          "capabilities": {
            "chat": true,
            "tool_call": true
          }
        }
      ]
    }
  }
}"#,
        )
        .expect("legacy fixture should be written");

        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");

        let setting_repo = ProviderSettingRepository::new(&store);
        let model_repo = ProviderModelRepository::new(&store);

        let settings = setting_repo.list().expect("legacy settings should migrate");
        let models = model_repo.list().expect("legacy models should migrate");

        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].provider_id, "deepseek");
        assert_eq!(settings[0].api_key, "sk-legacy");
        assert_eq!(settings[0].base_url, "https://api.deepseek.com/v1");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider_id, "deepseek");
        assert_eq!(models[0].model_id, "deepseek-chat");
        assert!(models[0].capabilities.chat);
        assert!(models[0].capabilities.tool_call);
        assert!(!models[0].capabilities.reasoning);
    }

    #[test]
    fn model_repository_migrates_flat_provider_config_shape() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let legacy_path = temp_dir.path().join("provider_configs.json");

        fs::write(
            &legacy_path,
            r#"{
  "deepseek": {
    "provider_id": "deepseek",
    "enabled": true,
    "api_key": "sk-legacy",
    "base_url": "https://api.deepseek.com/v1",
    "models": [
      {
        "model_id": "deepseek-chat",
        "capabilities": {
          "chat": true,
          "reasoning": true
        }
      }
    ]
  }
}"#,
        )
        .expect("flat legacy fixture should be written");

        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");

        let model_repo = ProviderModelRepository::new(&store);
        let models = model_repo.list().expect("flat legacy models should migrate");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider_id, "deepseek");
        assert_eq!(models[0].model_id, "deepseek-chat");
        assert!(models[0].capabilities.chat);
        assert!(models[0].capabilities.reasoning);
    }
}

use std::cell::RefCell;
use std::collections::HashMap;

use crate::db::local_store::LocalJsonStore;
use crate::models::default_model::DefaultModelSetting;

const FILENAME: &str = "default_model_settings";

pub struct DefaultModelSettingRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, DefaultModelSetting>>,
}

impl<'a> DefaultModelSettingRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let cache = store
            .read(FILENAME)
            .unwrap_or_else(|_| HashMap::default());

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<DefaultModelSetting>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(results)
    }

    pub fn upsert(&self, setting: &DefaultModelSetting) -> Result<String, String> {
        if setting.key.trim().is_empty() {
            return Err("key 不能为空".to_string());
        }

        if setting.provider_id.trim().is_empty() {
            return Err("provider_id 不能为空".to_string());
        }

        if setting.model_id.trim().is_empty() {
            return Err("model_id 不能为空".to_string());
        }

        let key = setting.key.clone();
        self.cache.borrow_mut().insert(key.clone(), setting.clone());
        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::DefaultModelSettingRepository;
    use crate::db::local_store::LocalJsonStore;
    use crate::models::default_model::DefaultModelSetting;

    #[test]
    fn upsert_persists_default_model_setting() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = DefaultModelSettingRepository::new(&store);

        let setting = DefaultModelSetting {
            key: "assistant".to_string(),
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
            temperature: "0.7".to_string(),
            top_p: "0.9".to_string(),
            max_tokens: "4096".to_string(),
            presence_penalty: "0".to_string(),
            frequency_penalty: "0".to_string(),
        };

        repository
            .upsert(&setting)
            .expect("default model setting should persist");

        let persisted_path = temp_dir.path().join("default_model_settings.json");
        assert!(persisted_path.exists());

        let content = fs::read_to_string(&persisted_path)
            .expect("persisted default model settings should be readable");
        assert!(content.contains("\"assistant\""));
        assert!(content.contains("\"provider_id\""));
        assert!(content.contains("\"top_p\""));
    }
}

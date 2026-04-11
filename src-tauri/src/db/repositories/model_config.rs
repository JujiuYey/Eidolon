use std::cell::RefCell;
use std::collections::HashMap;

use crate::db::local_store::{now, LocalJsonStore};
use crate::models::model_config::ModelProviderSettingDoc;

const FILENAME: &str = "model_provider_settings";

/// 模型配置仓储
pub struct ModelConfigRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, ModelProviderSettingDoc>>,
}

impl<'a> ModelConfigRepository<'a> {
    /// 创建新的仓储实例
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let cache = store
            .read(FILENAME)
            .unwrap_or_else(|_| HashMap::default());

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    /// 列出所有配置
    pub fn list(&self) -> Result<Vec<ModelProviderSettingDoc>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|a, b| {
            let a_key = a.provider_key.as_str();
            let b_key = b.provider_key.as_str();
            a_key.cmp(b_key)
        });
        Ok(results)
    }

    /// 创建或更新配置
    pub fn upsert(&self, config: &ModelProviderSettingDoc) -> Result<String, String> {
        let key = config.provider_key.as_str().to_string();

        let mut doc = config.clone();
        if doc.created_at.is_none() {
            doc.created_at = Some(now());
        }
        doc.updated_at = Some(now());

        // 更新缓存
        self.cache.borrow_mut().insert(key.clone(), doc);

        // 持久化
        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(key)
    }

    /// 更新配置
    pub fn update(&self, config: &ModelProviderSettingDoc) -> Result<(), String> {
        let key = config.provider_key.as_str();

        if !self.cache.borrow().contains_key(key) {
            return Err(format!(
                "未找到 provider_key 为 {} 的模型服务配置",
                key
            ));
        }

        let mut doc = config.clone();
        doc.updated_at = Some(now());

        self.cache.borrow_mut().insert(key.to_string(), doc);
        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(())
    }

    /// 删除配置
    pub fn delete(&self, id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(id).is_none() {
            return Err(format!("未找到 provider_key 为 {} 的模型服务配置", id));
        }

        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(id.to_string())
    }

    /// 设置默认配置（兼容旧接口）
    pub fn set_default(&self, _id: &str) -> Result<String, String> {
        Err("当前模型服务结构不再在 model_provider_settings 中维护默认平台，请改由独立设置文档保存".to_string())
    }
}

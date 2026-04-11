use crate::db::mongodb::AppMongoDb;
use crate::models::model_config::ModelProviderSettingDoc;
use mongodb::{
    bson::{doc, DateTime},
    options::FindOptions,
    Collection,
};

const COLLECTION_NAME: &str = "model_provider_settings";

pub struct ModelConfigRepository<'a> {
    coll: Collection<ModelProviderSettingDoc>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> ModelConfigRepository<'a> {
    pub fn new(db: &'a AppMongoDb) -> Self {
        Self {
            coll: db.collection::<ModelProviderSettingDoc>(COLLECTION_NAME),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn list(&self) -> Result<Vec<ModelProviderSettingDoc>, String> {
        let filter = doc! {};
        let options = FindOptions::builder().sort(doc! { "_id": 1 }).build();

        let mut cursor = self
            .coll
            .find(filter)
            .with_options(options)
            .await
            .map_err(|error| format!("查询模型服务配置失败: {error}"))?;

        let mut results = Vec::new();
        while cursor
            .advance()
            .await
            .map_err(|error| format!("读取游标失败: {error}"))?
        {
            results.push(
                cursor
                    .deserialize_current()
                    .map_err(|error| format!("反序列化失败: {error}"))?,
            );
        }

        Ok(results)
    }

    pub async fn create(&self, config: &ModelProviderSettingDoc) -> Result<String, String> {
        let now = DateTime::now();
        let mut doc = config.clone();
        doc.created_at = Some(now.clone());
        doc.updated_at = Some(now);

        self.coll
            .insert_one(doc)
            .await
            .map_err(|error| format!("创建模型服务配置失败: {error}"))?;

        Ok(config.provider_key.as_str().to_string())
    }

    pub async fn update(&self, config: &ModelProviderSettingDoc) -> Result<(), String> {
        let filter = doc! { "_id": config.provider_key.as_str() };
        let mut replacement = config.clone();
        replacement.updated_at = Some(DateTime::now());

        let result = self
            .coll
            .replace_one(filter, replacement)
            .await
            .map_err(|error| format!("更新模型服务配置失败: {error}"))?;

        if result.matched_count == 0 {
            return Err(format!(
                "未找到 provider_key 为 {} 的模型服务配置",
                config.provider_key.as_str()
            ));
        }

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<String, String> {
        let filter = doc! { "_id": id };

        let result = self
            .coll
            .delete_one(filter)
            .await
            .map_err(|error| format!("删除模型服务配置失败: {error}"))?;

        if result.deleted_count == 0 {
            return Err(format!("未找到 provider_key 为 {} 的模型服务配置", id));
        }

        Ok(id.to_string())
    }

    pub async fn set_default(&self, _id: &str) -> Result<String, String> {
        Err("当前模型服务结构不再在 model_provider_settings 中维护默认平台，请改由独立设置文档保存".to_string())
    }
}

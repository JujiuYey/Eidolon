use mongodb::{options::ClientOptions, Client, Collection, Database};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::time::Duration;

pub struct AppMongoDb {
    pub client: Client,
    pub database: Database,
}

fn resolve_mongodb_url(search_root: &Path) -> Result<String, String> {
    if let Some(mongodb_url) = std::env::var("MONGODB_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(mongodb_url);
    }

    let candidates = [
        search_root.join("src-tauri").join(".env"),
        search_root.join(".env"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            dotenvy::from_path(candidate)
                .map_err(|error| format!("加载环境变量文件失败 ({}): {error}", candidate.display()))?;
            if let Some(url) = std::env::var("MONGODB_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                return Ok(url);
            }
        }
    }

    Err("MONGODB_URL 环境变量未设置，请在 src-tauri/.env 中配置".to_string())
}

impl AppMongoDb {
    pub async fn open() -> Result<Self, String> {
        let search_root = std::env::current_dir()
            .map_err(|error| format!("读取当前工作目录失败: {error}"))?;
        let mongodb_url = resolve_mongodb_url(&search_root)?;

        let mut client_options = ClientOptions::parse(&mongodb_url)
            .await
            .map_err(|error| format!("解析 MongoDB URL 失败: {error}"))?;

        client_options.connect_timeout = Some(Duration::from_secs(8));
        client_options.server_selection_timeout = Some(Duration::from_secs(8));

        let client = Client::with_options(client_options)
            .map_err(|error| format!("连接 MongoDB 失败: {error}"))?;

        let database_name = std::env::var("MONGODB_DATABASE")
            .unwrap_or_else(|_| "sco_code_app".to_string());

        let database = client.database(&database_name);

        Ok(Self { client, database })
    }

    pub fn collection<T: Serialize + DeserializeOwned + Send + Sync>(
        &self,
        name: &str,
    ) -> Collection<T> {
        self.database.collection(name)
    }
}

pub mod repositories;
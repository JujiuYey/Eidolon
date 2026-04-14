use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::model_config::ProviderConfigRepository;
use crate::models::model_config::ProviderConfig;
use serde::Deserialize;

#[tauri::command]
pub fn list_provider_configs(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<ProviderConfig>, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn upsert_provider_config(
    store: tauri::State<'_, LocalJsonStore>,
    config: ProviderConfig,
) -> Result<String, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.upsert(&config)
}

#[tauri::command]
pub fn delete_provider_config(
    store: tauri::State<'_, LocalJsonStore>,
    provider_id: String,
) -> Result<String, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.delete(&provider_id)
}

/// 从 OpenAI 兼容接口（GET /models）或 Ollama Tags API 拉取模型列表。
#[tauri::command]
pub async fn fetch_provider_models(
    base_url: String,
    api_key: String,
    api_type: String,
) -> Result<Vec<String>, String> {
    if api_type == "ollama" {
        fetch_ollama_models(&base_url).await
    } else {
        fetch_openai_models(&base_url, &api_key).await
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

async fn fetch_openai_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let mut request = client.get(&url);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("请求失败: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("接口返回错误: {}", response.status()));
    }

    let body: OpenAIModelsResponse = response
        .json()
        .await
        .map_err(|error| format!("解析响应失败: {error}"))?;

    let mut ids: Vec<String> = body.data.into_iter().map(|model| model.id).collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("请求失败: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("接口返回错误: {}", response.status()));
    }

    let body: OllamaTagsResponse = response
        .json()
        .await
        .map_err(|error| format!("解析响应失败: {error}"))?;

    let mut names: Vec<String> = body.models.into_iter().map(|model| model.name).collect();
    names.sort();
    names.dedup();
    Ok(names)
}

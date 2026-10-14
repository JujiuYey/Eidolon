use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// 单请求超时：模型列表接口通常很快，超过 20 秒视为异常
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// 上游返回的单个模型条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedModel {
    pub model_id: String,
}

/// OpenAI 兼容响应：`{ "data": [{ "id": "gpt-4o" }] }`
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    #[serde(default)]
    data: Vec<OpenAiModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelEntry {
    id: String,
}

/// Ollama 响应：`{ "models": [{ "name": "llama3:8b" }] }`
#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    #[serde(default)]
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
}

/// 拉取某个厂商当前可用的模型 ID 列表。
///
/// 只负责取回远端清单，不写库：能力标记仍由用户在设置页逐个确认。
pub async fn fetch_models(
    base_url: &str,
    api_key: &str,
    provider_id: &str,
) -> Result<Vec<FetchedModel>, String> {
    let url = build_models_url(base_url, provider_id)?;

    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // 根地址可能重定向到子路径（如自建网关），允许有限跟随
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))?;

    let mut request = client.get(&url);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|error| request_error(&error, &url))?
        .error_for_status()
        .map_err(|error| status_error(&error, &url))?;

    let body = response
        .text()
        .await
        .map_err(|error| format!("读取模型列表失败: {error}"))?;

    let mut model_ids = parse_model_ids(provider_id, &body)?;

    if model_ids.is_empty() {
        return Err(format!(
            "接口返回成功但没有解析出模型，请确认 API 地址指向 OpenAI 兼容服务的 /v1 路径；当前地址: {url}"
        ));
    }

    // 去重并稳定排序，避免上游顺序抖动导致界面跳动
    model_ids.sort_unstable();
    model_ids.dedup();

    Ok(model_ids
        .into_iter()
        .map(|model_id| FetchedModel { model_id })
        .collect())
}

/// Ollama 用 /api/tags，其余厂商走 OpenAI 兼容的 /models
fn build_models_url(base_url: &str, provider_id: &str) -> Result<String, String> {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return Err("请先填写 API 地址".to_string());
    }

    Ok(if provider_id == "ollama" {
        format!("{base_url}/api/tags")
    } else {
        format!("{base_url}/models")
    })
}

/// 兼容两种响应结构：OpenAI 的 `data[].id` 与 Ollama 的 `models[].name`
fn parse_model_ids(provider_id: &str, body: &str) -> Result<Vec<String>, String> {
    if provider_id == "ollama" {
        let parsed: OllamaModelsResponse = serde_json::from_str(body)
            .map_err(|error| format!("解析 Ollama 模型列表失败: {error}"))?;
        return Ok(parsed
            .models
            .into_iter()
            .map(|entry| entry.name)
            .filter(|name| !name.trim().is_empty())
            .collect());
    }

    let parsed: OpenAiModelsResponse =
        serde_json::from_str(body).map_err(|error| format!("解析模型列表失败: {error}"))?;
    Ok(parsed
        .data
        .into_iter()
        .map(|entry| entry.id)
        .filter(|id| !id.trim().is_empty())
        .collect())
}

fn request_error(error: &reqwest::Error, url: &str) -> String {
    let message = format!("请求模型列表失败: {error}");
    if error.is_connect() {
        #[cfg(target_os = "macos")]
        return format!(
            "{message}。若为本地服务（如 Ollama），请在 系统设置 → 隐私与安全性 → 本地网络 中允许本应用"
        );
    }
    format!("{message}（请求地址: {url}）")
}

fn status_error(error: &reqwest::Error, url: &str) -> String {
    match error.status() {
        Some(status)
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN =>
        {
            format!(
                "API 密钥无效或无权限访问模型列表（HTTP {}）: {url}",
                status.as_u16()
            )
        }
        Some(status) if status == reqwest::StatusCode::NOT_FOUND => {
            format!("接口不存在（HTTP 404）: {url}。请确认 API 地址是否需要以 /v1 结尾")
        }
        _ => format!("请求模型列表失败: {error}（请求地址: {url}）"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_shape() {
        let ids = parse_model_ids(
            "deepseek",
            r#"{"data":[{"id":"deepseek-chat"},{"id":"deepseek-reasoner"}]}"#,
        )
        .expect("parse");
        assert_eq!(ids, vec!["deepseek-chat", "deepseek-reasoner"]);
    }

    #[test]
    fn parses_ollama_shape() {
        let ids = parse_model_ids(
            "ollama",
            r#"{"models":[{"name":"llama3:8b"},{"name":"qwen2:7b"}]}"#,
        )
        .expect("parse");
        assert_eq!(ids, vec!["llama3:8b", "qwen2:7b"]);
    }

    #[test]
    fn parses_empty_data() {
        let ids = parse_model_ids("minimax", r#"{"data":[]}"#).expect("parse");
        assert!(ids.is_empty());
    }

    #[test]
    fn rejects_blank_base_url() {
        assert_eq!(
            build_models_url("   ", "deepseek").unwrap_err(),
            "请先填写 API 地址"
        );
    }

    #[test]
    fn trims_trailing_slash_before_appending_path() {
        assert_eq!(
            build_models_url("https://api.deepseek.com/v1/", "deepseek").unwrap(),
            "https://api.deepseek.com/v1/models"
        );
        assert_eq!(
            build_models_url("http://127.0.0.1:11434", "ollama").unwrap(),
            "http://127.0.0.1:11434/api/tags"
        );
    }

    /// 起一个本地 OpenAI 兼容端点，验证真实请求：路径、鉴权头、排序去重
    #[tokio::test]
    async fn fetch_models_against_local_server() {
        use axum::extract::State;
        use axum::http::{HeaderMap, StatusCode};
        use axum::routing::get;
        use axum::Router;

        async fn list_models(
            State(sender): State<tokio::sync::mpsc::UnboundedSender<String>>,
            headers: HeaderMap,
        ) -> (StatusCode, String) {
            let auth = headers
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let _ = sender.send(auth);
            (
                StatusCode::OK,
                r#"{"object":"list","data":[{"id":"zeta"},{"id":"alpha"},{"id":"alpha"}]}"#
                    .to_string(),
            )
        }

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let app = Router::new()
            .route("/v1/models", get(list_models))
            .with_state(sender);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let models = fetch_models(&format!("http://{addr}/v1"), "sk-test", "deepseek")
            .await
            .expect("fetch");

        // 排序 + 去重
        assert_eq!(
            models
                .iter()
                .map(|m| m.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        // Bearer 鉴权头确实带上了
        assert_eq!(
            receiver.recv().await.as_deref(),
            Some("Bearer sk-test"),
            "服务端未收到 Bearer 鉴权头"
        );

        server.abort();
    }

    /// 401 要给出可操作的中文提示，而不是裸的 HTTP 状态码
    #[tokio::test]
    async fn fetch_models_reports_unauthorized() {
        use axum::http::StatusCode;
        use axum::routing::get;
        use axum::Router;

        let app = Router::new().route(
            "/v1/models",
            get(|| async { (StatusCode::UNAUTHORIZED, "nope".to_string()) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let error = fetch_models(&format!("http://{addr}/v1"), "", "deepseek")
            .await
            .unwrap_err();
        assert!(error.contains("API 密钥无效"), "unexpected: {error}");

        server.abort();
    }

    /// 空列表要报错而不是静默返回空数组
    #[tokio::test]
    async fn fetch_models_rejects_empty_list() {
        use axum::routing::get;
        use axum::Router;

        let app = Router::new().route(
            "/v1/models",
            get(|| async { r#"{"object":"list","data":[]}"#.to_string() }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let error = fetch_models(&format!("http://{addr}/v1"), "sk", "deepseek")
            .await
            .unwrap_err();
        assert!(error.contains("没有解析出模型"), "unexpected: {error}");

        server.abort();
    }
}

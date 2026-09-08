use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use nanoid::nanoid;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Method, Response};
use tokio::sync::watch;
use url::Url;

use crate::models::api_client::{
    normalize_method, ApiEnvironment, BodyKind, ExecutionResult, ExecutionStatus, KeyValueRow,
    RequestSnapshot, MAX_RESPONSE_BYTES,
};

/// 构建好的实际请求，变量已解析
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<KeyValueRow>,
    pub body: Option<Vec<u8>>,
    pub timeout: Duration,
}

/// 执行中的请求登记表，按执行 ID 关联取消信号
#[derive(Default)]
pub struct ExecutionRegistry {
    inflight: Mutex<HashMap<String, watch::Sender<bool>>>,
}

impl ExecutionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, execution_id: &str) -> Result<watch::Receiver<bool>, String> {
        let (sender, receiver) = watch::channel(false);
        let mut guard = self
            .inflight
            .lock()
            .map_err(|_| "执行状态已损坏".to_string())?;
        guard.insert(execution_id.to_string(), sender);
        Ok(receiver)
    }

    fn unregister(&self, execution_id: &str) {
        if let Ok(mut guard) = self.inflight.lock() {
            guard.remove(execution_id);
        }
    }

    /// 请求客户端停止等待。服务端可能已经执行，取消不代表已回滚。
    pub fn cancel(&self, execution_id: &str) -> Result<bool, String> {
        let guard = self
            .inflight
            .lock()
            .map_err(|_| "执行状态已损坏".to_string())?;

        match guard.get(execution_id) {
            Some(sender) => {
                let _ = sender.send(true);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn is_inflight(&self, execution_id: &str) -> bool {
        self.inflight
            .lock()
            .map(|guard| guard.contains_key(execution_id))
            .unwrap_or(false)
    }
}

/// 共享 HTTP 客户端。不自动跟随重定向，不持久化 Cookie，不自动重试。
pub struct ApiHttpClient {
    client: Client,
    registry: Arc<ExecutionRegistry>,
}

impl ApiHttpClient {
    pub fn new() -> Result<Self, String> {
        // 默认即不持久化 Cookie：不启用 `cookies` 特性，不构造 CookieStore。
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))?;

        Ok(Self {
            client,
            registry: Arc::new(ExecutionRegistry::new()),
        })
    }

    pub fn registry(&self) -> Arc<ExecutionRegistry> {
        Arc::clone(&self.registry)
    }

    /// 发送一次请求。4xx/5xx 是正常响应；连接失败、超时、取消是执行错误。
    pub async fn send(
        &self,
        execution_id: &str,
        prepared: &PreparedRequest,
    ) -> Result<ExecutionResult, String> {
        let mut cancel = self.registry.register(execution_id)?;
        let started = Instant::now();

        let outcome = self
            .execute(prepared, execution_id, started, &mut cancel)
            .await;

        self.registry.unregister(execution_id);
        outcome
    }

    async fn execute(
        &self,
        prepared: &PreparedRequest,
        execution_id: &str,
        started: Instant,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<ExecutionResult, String> {
        let method = Method::from_bytes(prepared.method.as_bytes())
            .map_err(|error| format!("无法识别的 HTTP 方法: {error}"))?;

        let mut builder = self
            .client
            .request(method, &prepared.url)
            .timeout(prepared.timeout);

        builder = builder.headers(build_header_map(&prepared.headers)?);

        if let Some(body) = prepared.body.clone() {
            builder = builder.body(body);
        }

        let send_future = builder.send();

        let response = tokio::select! {
            biased;
            _ = cancel.changed() => {
                return Ok(cancelled_result(execution_id, started));
            }
            result = send_future => result,
        };

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(error_result(execution_id, started, error));
            }
        };

        self.read_response(execution_id, response, started, cancel)
            .await
    }

    /// 流式读取响应，超过上限即终止，不做无上限缓冲
    async fn read_response(
        &self,
        execution_id: &str,
        response: Response,
        started: Instant,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<ExecutionResult, String> {
        let status = response.status();
        let response_headers = collect_headers(response.headers());
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);

        let mut buffer: Vec<u8> = Vec::new();
        let mut oversized = false;
        let mut stream = response.bytes_stream();

        loop {
            let chunk = tokio::select! {
                biased;
                _ = cancel.changed() => {
                    return Ok(cancelled_result(execution_id, started));
                }
                chunk = stream.next() => chunk,
            };

            let Some(chunk) = chunk else {
                break;
            };

            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    return Ok(ExecutionResult {
                        response_headers,
                        status_code: Some(status.as_u16()),
                        ..error_result(execution_id, started, error)
                    });
                }
            };

            let remaining = MAX_RESPONSE_BYTES.saturating_sub(buffer.len());
            if chunk.len() > remaining {
                buffer.extend_from_slice(&chunk[..remaining]);
                oversized = true;
                break;
            }

            buffer.extend_from_slice(&chunk);

            if buffer.len() >= MAX_RESPONSE_BYTES {
                oversized = true;
                break;
            }
        }

        let body_size_bytes = buffer.len();
        let is_binary = is_binary_response(content_type.as_deref(), &buffer);
        let body_text = if is_binary {
            String::new()
        } else {
            String::from_utf8_lossy(&buffer).to_string()
        };

        let status_kind = if oversized {
            ExecutionStatus::Oversize
        } else if status.is_client_error() || status.is_server_error() {
            ExecutionStatus::HttpError
        } else {
            ExecutionStatus::Success
        };

        Ok(ExecutionResult {
            execution_id: execution_id.to_string(),
            status: status_kind,
            status_code: Some(status.as_u16()),
            response_headers,
            body_text,
            body_size_bytes,
            content_type,
            is_binary,
            is_oversized: oversized,
            duration_ms: started.elapsed().as_millis() as u64,
            error_message: if oversized {
                Some(format!(
                    "响应超过上限（{} MiB），已停止继续读取",
                    MAX_RESPONSE_BYTES / (1024 * 1024)
                ))
            } else {
                None
            },
            history_error: None,
            history_id: None,
        })
    }
}

/// 生成一个执行 ID
pub fn new_execution_id() -> String {
    format!("aexe_{}", nanoid!(12))
}

fn cancelled_result(execution_id: &str, started: Instant) -> ExecutionResult {
    ExecutionResult {
        execution_id: execution_id.to_string(),
        status: ExecutionStatus::Cancelled,
        status_code: None,
        response_headers: Vec::new(),
        body_text: String::new(),
        body_size_bytes: 0,
        content_type: None,
        is_binary: false,
        is_oversized: false,
        duration_ms: started.elapsed().as_millis() as u64,
        error_message: Some("已取消等待，服务端可能已经执行".to_string()),
        history_error: None,
        history_id: None,
    }
}

fn error_result(execution_id: &str, started: Instant, error: reqwest::Error) -> ExecutionResult {
    let status = if error.is_timeout() {
        ExecutionStatus::Timeout
    } else {
        ExecutionStatus::NetworkError
    };

    let message = if error.is_timeout() {
        "请求超时".to_string()
    } else if error.is_connect() {
        format!("连接失败: {error}")
    } else {
        format!("请求失败: {error}")
    };

    ExecutionResult {
        execution_id: execution_id.to_string(),
        status,
        status_code: None,
        response_headers: Vec::new(),
        body_text: String::new(),
        body_size_bytes: 0,
        content_type: None,
        is_binary: false,
        is_oversized: false,
        duration_ms: started.elapsed().as_millis() as u64,
        error_message: Some(message),
        history_error: None,
        history_id: None,
    }
}

fn build_header_map(headers: &[KeyValueRow]) -> Result<HeaderMap, String> {
    let mut map = HeaderMap::new();

    for row in headers.iter().filter(|row| row.enabled) {
        let name = row.key.trim();
        if name.is_empty() {
            continue;
        }

        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| format!("请求头名称不合法: {name}"))?;
        let header_value = HeaderValue::from_str(row.value.trim())
            .map_err(|_| format!("请求头 {name} 的值包含不支持的字符"))?;

        map.append(header_name, header_value);
    }

    Ok(map)
}

fn collect_headers(headers: &HeaderMap) -> Vec<KeyValueRow> {
    headers
        .iter()
        .map(|(name, value)| KeyValueRow {
            id: String::new(),
            enabled: true,
            key: name.as_str().to_string(),
            value: value.to_str().unwrap_or("<非文本值>").to_string(),
        })
        .collect()
}

/// 二进制响应仅提示类型和大小，不强制转换为文本
fn is_binary_response(content_type: Option<&str>, body: &[u8]) -> bool {
    if let Some(content_type) = content_type {
        let lower = content_type.to_ascii_lowercase();
        let textual = lower.starts_with("text/")
            || lower.contains("json")
            || lower.contains("xml")
            || lower.contains("javascript")
            || lower.contains("x-www-form-urlencoded");

        if textual {
            return false;
        }

        if lower.starts_with("image/")
            || lower.starts_with("audio/")
            || lower.starts_with("video/")
            || lower.starts_with("application/octet-stream")
            || lower.starts_with("application/pdf")
            || lower.starts_with("application/zip")
        {
            return true;
        }
    }

    // 未声明类型时按内容判断：出现 NUL 字节或无法解码为 UTF-8 视为二进制
    body.contains(&0) || std::str::from_utf8(body).is_err()
}

/// 变量解析与请求构建的错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareError {
    /// 存在未定义变量，列出缺失变量名
    MissingVariables(Vec<String>),
    Invalid(String),
}

impl PrepareError {
    pub fn message(&self) -> String {
        match self {
            PrepareError::MissingVariables(names) => {
                format!("存在未定义变量: {}", names.join(", "))
            }
            PrepareError::Invalid(message) => message.clone(),
        }
    }
}

impl From<PrepareError> for String {
    fn from(error: PrepareError) -> Self {
        error.message()
    }
}

/// 变量表：环境变量一级作用域，首版没有多层覆盖
pub struct VariableScope {
    values: HashMap<String, String>,
    base_url: String,
}

impl VariableScope {
    pub fn from_environment(environment: Option<&ApiEnvironment>) -> Self {
        let mut values = HashMap::new();
        let mut base_url = String::new();

        if let Some(environment) = environment {
            base_url = environment.base_url.trim().to_string();
            if !base_url.is_empty() {
                values.insert("base_url".to_string(), base_url.clone());
            }

            for variable in environment.variables.iter().filter(|row| row.enabled) {
                let key = variable.key.trim();
                if key.is_empty() {
                    continue;
                }
                values.insert(key.to_string(), variable.value.clone());
            }
        }

        Self { values, base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.values.get(name)
    }
}

/// `{{变量名}}` 文本替换。首版不推断或转换类型。
pub fn resolve_variables(input: &str, scope: &VariableScope) -> Result<String, PrepareError> {
    let mut output = String::with_capacity(input.len());
    let mut missing: Vec<String> = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let Some(end) = after.find("}}") else {
            output.push_str(&rest[start..]);
            rest = "";
            break;
        };

        let name = after[..end].trim();
        match scope.get(name) {
            Some(value) => output.push_str(value),
            None => {
                if !missing.iter().any(|existing| existing == name) {
                    missing.push(name.to_string());
                }
            }
        }

        rest = &after[end + 2..];
    }

    output.push_str(rest);

    if !missing.is_empty() {
        return Err(PrepareError::MissingVariables(missing));
    }

    Ok(output)
}

fn resolve_rows(
    rows: &[KeyValueRow],
    scope: &VariableScope,
    missing: &mut Vec<String>,
) -> Vec<KeyValueRow> {
    rows.iter()
        .filter(|row| row.enabled)
        .map(|row| {
            let key = collect_missing(resolve_variables(&row.key, scope), missing)
                .unwrap_or_else(|| row.key.clone());
            let value = collect_missing(resolve_variables(&row.value, scope), missing)
                .unwrap_or_else(|| row.value.clone());

            KeyValueRow {
                id: row.id.clone(),
                enabled: true,
                key,
                value,
            }
        })
        .collect()
}

fn collect_missing(
    result: Result<String, PrepareError>,
    missing: &mut Vec<String>,
) -> Option<String> {
    match result {
        Ok(value) => Some(value),
        Err(PrepareError::MissingVariables(names)) => {
            for name in names {
                if !missing.iter().any(|existing| existing == &name) {
                    missing.push(name);
                }
            }
            None
        }
        Err(PrepareError::Invalid(_)) => None,
    }
}

/// 由请求快照与环境构建实际请求：解析变量、拼接 Query、补充 Content-Type
pub fn prepare_request(
    snapshot: &RequestSnapshot,
    environment: Option<&ApiEnvironment>,
) -> Result<PreparedRequest, PrepareError> {
    let scope = VariableScope::from_environment(environment);
    let mut missing: Vec<String> = Vec::new();

    let method = normalize_method(&snapshot.method).map_err(PrepareError::Invalid)?;

    let raw_url =
        collect_missing(resolve_variables(&snapshot.url, &scope), &mut missing).unwrap_or_default();
    let headers = resolve_rows(&snapshot.headers, &scope, &mut missing);
    let query = resolve_rows(&snapshot.query, &scope, &mut missing);

    let body_text = match snapshot.body.kind {
        BodyKind::None => String::new(),
        BodyKind::Json | BodyKind::Text => {
            collect_missing(resolve_variables(&snapshot.body.text, &scope), &mut missing)
                .unwrap_or_default()
        }
        BodyKind::Form => String::new(),
    };

    let form_rows = if snapshot.body.kind == BodyKind::Form {
        resolve_rows(&snapshot.body.form, &scope, &mut missing)
    } else {
        Vec::new()
    };

    if !missing.is_empty() {
        return Err(PrepareError::MissingVariables(missing));
    }

    // JSON 模式先替换变量再做语法校验
    if snapshot.body.kind == BodyKind::Json && !body_text.trim().is_empty() {
        serde_json::from_str::<serde_json::Value>(&body_text)
            .map_err(|error| PrepareError::Invalid(format!("请求体不是合法 JSON: {error}")))?;
    }

    let mut url = build_url(&raw_url, scope.base_url())?;
    append_query(&mut url, &query);

    if !matches!(url.scheme(), "http" | "https") {
        return Err(PrepareError::Invalid(format!(
            "只支持 HTTP 和 HTTPS，收到: {}",
            url.scheme()
        )));
    }

    let body = match snapshot.body.kind {
        BodyKind::None => None,
        BodyKind::Json | BodyKind::Text => {
            if body_text.is_empty() {
                None
            } else {
                Some(body_text.into_bytes())
            }
        }
        BodyKind::Form => {
            let encoded = form_urlencoded_body(&form_rows);
            if encoded.is_empty() {
                None
            } else {
                Some(encoded.into_bytes())
            }
        }
    };

    let headers = with_default_content_type(headers, snapshot.body.kind, body.is_some());

    Ok(PreparedRequest {
        method,
        url: url.to_string(),
        headers,
        body,
        timeout: Duration::from_millis(snapshot.timeout_ms.max(1)),
    })
}

/// 绝对 URL 直接使用；相对路径通过所选环境的基础地址解析
fn build_url(raw_url: &str, base_url: &str) -> Result<Url, PrepareError> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(PrepareError::Invalid("请求地址不能为空".to_string()));
    }

    if let Ok(url) = Url::parse(trimmed) {
        if url.has_host() {
            return Ok(url);
        }
    }

    let base = base_url.trim();
    if base.is_empty() {
        return Err(PrepareError::Invalid(
            "相对地址需要所选环境提供基础地址".to_string(),
        ));
    }

    let base_with_slash = if base.ends_with('/') {
        base.to_string()
    } else {
        format!("{base}/")
    };

    let base_url = Url::parse(&base_with_slash)
        .map_err(|error| PrepareError::Invalid(format!("基础地址不合法: {error}")))?;

    base_url
        .join(trimmed.trim_start_matches('/'))
        .map_err(|error| PrepareError::Invalid(format!("请求地址不合法: {error}")))
}

/// 追加 Query 参数，保留顺序和重复键
fn append_query(url: &mut Url, query: &[KeyValueRow]) {
    let rows: Vec<&KeyValueRow> = query
        .iter()
        .filter(|row| !row.key.trim().is_empty())
        .collect();

    if rows.is_empty() {
        return;
    }

    let mut pairs = url.query_pairs_mut();
    for row in rows {
        pairs.append_pair(row.key.trim(), &row.value);
    }
}

fn form_urlencoded_body(rows: &[KeyValueRow]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for row in rows.iter().filter(|row| !row.key.trim().is_empty()) {
        serializer.append_pair(row.key.trim(), &row.value);
    }
    serializer.finish()
}

/// 按 Body 类型补充默认 Content-Type；用户显式设置时优先使用用户值
fn with_default_content_type(
    headers: Vec<KeyValueRow>,
    kind: BodyKind,
    has_body: bool,
) -> Vec<KeyValueRow> {
    if !has_body {
        return headers;
    }

    let Some(default_value) = kind.default_content_type() else {
        return headers;
    };

    let already_set = headers
        .iter()
        .any(|row| row.key.trim().eq_ignore_ascii_case("content-type"));

    if already_set {
        return headers;
    }

    let mut headers = headers;
    headers.push(KeyValueRow::new("Content-Type", default_value));
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api_client::RequestBody;

    fn environment(base_url: &str, variables: &[(&str, &str)]) -> ApiEnvironment {
        ApiEnvironment {
            id: "env1".to_string(),
            project_id: "p1".to_string(),
            name: "测试环境".to_string(),
            base_url: base_url.to_string(),
            variables: variables
                .iter()
                .map(|(key, value)| KeyValueRow::new(key, value))
                .collect(),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn snapshot() -> RequestSnapshot {
        RequestSnapshot {
            method: "GET".to_string(),
            url: "/orders".to_string(),
            query: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::default(),
            timeout_ms: 30_000,
            environment_name: Some("测试环境".to_string()),
        }
    }

    #[test]
    fn relative_url_resolves_through_environment_base_url() {
        let environment = environment("https://test.example.com/api", &[]);
        let prepared = prepare_request(&snapshot(), Some(&environment)).expect("prepare");

        assert_eq!(prepared.url, "https://test.example.com/api/orders");
        assert_eq!(prepared.method, "GET");
        assert!(prepared.body.is_none());
    }

    #[test]
    fn absolute_url_is_used_as_is() {
        let environment = environment("https://test.example.com", &[]);
        let prepared = prepare_request(
            &RequestSnapshot {
                url: "http://127.0.0.1:8080/health".to_string(),
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        assert_eq!(prepared.url, "http://127.0.0.1:8080/health");
    }

    #[test]
    fn variables_are_replaced_in_url_headers_query_and_body() {
        let environment = environment(
            "https://test.example.com",
            &[("token", "abc123"), ("tenant", "acme")],
        );

        let prepared = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                url: "{{base_url}}/tenants/{{tenant}}/orders".to_string(),
                headers: vec![KeyValueRow::new("Authorization", "Bearer {{token}}")],
                query: vec![KeyValueRow::new("tenant", "{{tenant}}")],
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{\"tenant\":\"{{tenant}}\"}".to_string(),
                    form: Vec::new(),
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        assert_eq!(
            prepared.url,
            "https://test.example.com/tenants/acme/orders?tenant=acme"
        );
        assert_eq!(
            prepared
                .headers
                .iter()
                .find(|row| row.key == "Authorization")
                .expect("auth header")
                .value,
            "Bearer abc123"
        );
        assert_eq!(
            String::from_utf8(prepared.body.expect("body")).expect("utf8"),
            "{\"tenant\":\"acme\"}"
        );
    }

    #[test]
    fn missing_variables_block_the_request_and_are_listed() {
        let environment = environment("https://test.example.com", &[]);

        let error = prepare_request(
            &RequestSnapshot {
                url: "{{base_url}}/orders/{{order_id}}".to_string(),
                headers: vec![KeyValueRow::new("X-Trace", "{{trace_id}}")],
                ..snapshot()
            },
            Some(&environment),
        )
        .expect_err("missing variables must block sending");

        match error {
            PrepareError::MissingVariables(names) => {
                assert!(names.contains(&"order_id".to_string()), "got {names:?}");
                assert!(names.contains(&"trace_id".to_string()), "got {names:?}");
            }
            other => panic!("expected missing variables, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_query_keys_and_disabled_rows_are_handled() {
        let environment = environment("https://test.example.com", &[]);

        let prepared = prepare_request(
            &RequestSnapshot {
                query: vec![
                    KeyValueRow::new("tag", "a"),
                    KeyValueRow::new("tag", "b"),
                    KeyValueRow::disabled("debug", "1"),
                ],
                headers: vec![
                    KeyValueRow::new("X-Multi", "one"),
                    KeyValueRow::new("X-Multi", "two"),
                    KeyValueRow::disabled("X-Skip", "no"),
                ],
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        assert!(
            prepared.url.ends_with("/orders?tag=a&tag=b"),
            "got {}",
            prepared.url
        );
        assert_eq!(prepared.headers.len(), 2, "disabled header must be dropped");

        let map = build_header_map(&prepared.headers).expect("header map");
        let values = map
            .get_all("x-multi")
            .iter()
            .map(|value| value.to_str().expect("utf8"))
            .collect::<Vec<_>>();
        assert_eq!(values, vec!["one", "two"], "duplicate headers must be kept");
    }

    #[test]
    fn json_body_is_validated_after_variable_substitution() {
        let environment = environment("https://test.example.com", &[("name", "订单")]);

        let error = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{\"name\": {{name}}}".to_string(),
                    form: Vec::new(),
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect_err("text substitution producing invalid JSON must be rejected");

        assert!(
            error.message().contains("合法 JSON"),
            "got {}",
            error.message()
        );

        let ok = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{\"name\": \"{{name}}\"}".to_string(),
                    form: Vec::new(),
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("quoted substitution is valid JSON");

        assert_eq!(
            String::from_utf8(ok.body.expect("body")).expect("utf8"),
            "{\"name\": \"订单\"}"
        );
    }

    #[test]
    fn default_content_type_is_added_only_when_absent() {
        let environment = environment("https://test.example.com", &[]);

        let prepared = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{}".to_string(),
                    form: Vec::new(),
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        assert_eq!(
            prepared
                .headers
                .iter()
                .find(|row| row.key.eq_ignore_ascii_case("content-type"))
                .expect("content type")
                .value,
            "application/json"
        );

        let explicit = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                headers: vec![KeyValueRow::new("content-type", "application/vnd.api+json")],
                body: RequestBody {
                    kind: BodyKind::Json,
                    text: "{}".to_string(),
                    form: Vec::new(),
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        let content_types = explicit
            .headers
            .iter()
            .filter(|row| row.key.eq_ignore_ascii_case("content-type"))
            .collect::<Vec<_>>();
        assert_eq!(content_types.len(), 1, "user value must win");
        assert_eq!(content_types[0].value, "application/vnd.api+json");
    }

    #[test]
    fn form_body_is_url_encoded_with_duplicate_keys() {
        let environment = environment("https://test.example.com", &[]);

        let prepared = prepare_request(
            &RequestSnapshot {
                method: "POST".to_string(),
                body: RequestBody {
                    kind: BodyKind::Form,
                    text: String::new(),
                    form: vec![
                        KeyValueRow::new("tag", "a b"),
                        KeyValueRow::new("tag", "c"),
                        KeyValueRow::disabled("skip", "1"),
                    ],
                },
                ..snapshot()
            },
            Some(&environment),
        )
        .expect("prepare");

        assert_eq!(
            String::from_utf8(prepared.body.expect("body")).expect("utf8"),
            "tag=a+b&tag=c"
        );
        assert_eq!(
            prepared
                .headers
                .iter()
                .find(|row| row.key.eq_ignore_ascii_case("content-type"))
                .expect("content type")
                .value,
            "application/x-www-form-urlencoded"
        );
    }

    #[test]
    fn non_http_schemes_are_rejected() {
        let error = prepare_request(
            &RequestSnapshot {
                url: "ftp://example.com/file".to_string(),
                ..snapshot()
            },
            None,
        )
        .expect_err("non-http scheme must be rejected");

        assert!(error.message().contains("HTTP"), "got {}", error.message());
    }

    #[test]
    fn relative_url_without_environment_is_rejected() {
        let error = prepare_request(&snapshot(), None).expect_err("relative url needs a base url");
        assert!(
            error.message().contains("基础地址"),
            "got {}",
            error.message()
        );
    }

    #[test]
    fn unsupported_method_is_rejected_before_sending() {
        let error = prepare_request(
            &RequestSnapshot {
                method: "TRACE".to_string(),
                url: "https://example.com".to_string(),
                ..snapshot()
            },
            None,
        )
        .expect_err("unsupported method must be rejected");

        assert!(error.message().contains("不支持的 HTTP 方法"));
    }

    #[test]
    fn binary_detection_uses_content_type_then_content() {
        assert!(is_binary_response(Some("image/png"), b"\x89PNG"));
        assert!(is_binary_response(Some("application/octet-stream"), b"abc"));
        assert!(!is_binary_response(Some("application/json"), b"{}"));
        assert!(!is_binary_response(
            Some("text/plain; charset=utf-8"),
            b"hello"
        ));
        assert!(is_binary_response(None, b"ab\0cd"));
        assert!(!is_binary_response(None, "中文".as_bytes()));
    }

    #[test]
    fn cancel_marks_unknown_execution_as_not_inflight() {
        let registry = ExecutionRegistry::new();
        assert!(!registry.is_inflight("missing"));
        assert!(!registry.cancel("missing").expect("cancel should not fail"));
    }

    #[test]
    fn resolve_variables_leaves_unclosed_placeholder_untouched() {
        let scope = VariableScope::from_environment(None);
        assert_eq!(
            resolve_variables("prefix {{unclosed", &scope).expect("no variable was resolved"),
            "prefix {{unclosed"
        );
    }
}

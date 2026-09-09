//! 请求准备：变量解析、URL 构造、Header / Query / Body 编码。
//! 纯数据变换层，不接触网络与 SQLite，便于单独测试。

use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;

use crate::models::api_client::{
    normalize_method, ApiEnvironment, BodyKind, KeyValueRow, RequestSnapshot,
};

/// 构建好的实际请求，变量已解析。
/// 由 [`prepare_request`] 返回，再交给 [`super::client::ApiHttpClient::send`] 执行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<KeyValueRow>,
    pub body: Option<Vec<u8>>,
    pub timeout: std::time::Duration,
}

/// 请求准备阶段抛出的错误。包含变量缺失与所有合法 JSON / URL / Method 校验失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareError {
    /// 存在未定义变量，列出缺失变量名
    MissingVariables(Vec<String>),
    /// 业务校验或格式错误（如方法、URL 协议、JSON 语法）
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

/// 变量表：环境变量一级作用域，首版没有多层覆盖。
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

/// `{{变量名}}` 文本替换。首版不推断或转换类型；字符串值需要正确的 JSON 引号与转义。
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

/// 由请求快照与环境构建实际请求：解析变量、拼接 Query、补充 Content-Type。
/// 出错时不会发出网络请求；所有错误以 [`PrepareError`] 返回。
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
        timeout: std::time::Duration::from_millis(snapshot.timeout_ms.max(1)),
    })
}

/// 解析键值行中的 `{{变量}}`，把失败收集到 `missing` 中；不阻断其他行的解析。
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

/// 把 prepared header 列表转换为 `HeaderMap`，丢弃 enabled=false、空键、含非法字符的值
pub fn build_header_map(headers: &[KeyValueRow]) -> Result<HeaderMap, String> {
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
    fn resolve_variables_leaves_unclosed_placeholder_untouched() {
        let scope = VariableScope::from_environment(None);
        assert_eq!(
            resolve_variables("prefix {{unclosed", &scope).expect("no variable was resolved"),
            "prefix {{unclosed"
        );
    }
}

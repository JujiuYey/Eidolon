use serde::{Deserialize, Serialize};

/// 默认请求超时（毫秒）
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// 响应读取上限：5 MiB
pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

/// 历史保存的响应体上限：1 MiB
pub const MAX_HISTORY_BODY_BYTES: usize = 1024 * 1024;

/// 每个请求保留的历史条数
pub const MAX_HISTORY_PER_REQUEST: usize = 100;

/// 请求体生成默认模型键，与聊天默认模型 `assistant` 相互独立
pub const BODY_GENERATION_MODEL_KEY: &str = "api_body_generation";

/// 可启用或禁用的键值行，允许重复键并保留顺序
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueRow {
    #[serde(default)]
    pub id: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub key: String,

    #[serde(default)]
    pub value: String,
}

impl KeyValueRow {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            id: String::new(),
            enabled: true,
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub fn disabled(key: &str, value: &str) -> Self {
        Self {
            enabled: false,
            ..Self::new(key, value)
        }
    }
}

/// 请求体类型
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyKind {
    #[default]
    None,
    Json,
    Text,
    Form,
}

impl BodyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BodyKind::None => "none",
            BodyKind::Json => "json",
            BodyKind::Text => "text",
            BodyKind::Form => "form",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "none" => Ok(BodyKind::None),
            "json" => Ok(BodyKind::Json),
            "text" => Ok(BodyKind::Text),
            "form" => Ok(BodyKind::Form),
            other => Err(format!("不支持的请求体类型: {other}")),
        }
    }

    /// 该类型的默认 Content-Type，`None` 表示不补充
    pub fn default_content_type(self) -> Option<&'static str> {
        match self {
            BodyKind::None => None,
            BodyKind::Json => Some("application/json"),
            BodyKind::Text => Some("text/plain; charset=utf-8"),
            BodyKind::Form => Some("application/x-www-form-urlencoded"),
        }
    }
}

/// 请求体：按类型保存文本或表单数组
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(default)]
    pub kind: BodyKind,

    #[serde(default)]
    pub text: String,

    #[serde(default)]
    pub form: Vec<KeyValueRow>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiProject {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiGroup {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub project_id: String,

    /// 预留多层分组，首版始终为 None
    #[serde(default)]
    pub parent_group_id: Option<String>,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// 请求定义。请求通过分组归属项目，`project_id` 为读取时补充的派生字段
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRequest {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub group_id: String,

    #[serde(default)]
    pub project_id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default = "default_method")]
    pub method: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub query: Vec<KeyValueRow>,

    #[serde(default)]
    pub headers: Vec<KeyValueRow>,

    #[serde(default)]
    pub body: RequestBody,

    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// AI 生成要求，随请求定义保存
    #[serde(default)]
    pub ai_prompt: String,

    /// AI 参考内容，随请求定义保存
    #[serde(default)]
    pub ai_reference: String,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEnvironment {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub project_id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub base_url: String,

    #[serde(default)]
    pub variables: Vec<KeyValueRow>,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// 执行状态。4xx/5xx 属于收到的响应，与连接失败、超时、取消区分
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    HttpError,
    NetworkError,
    Timeout,
    Cancelled,
    Oversize,
}

impl ExecutionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ExecutionStatus::Success => "success",
            ExecutionStatus::HttpError => "http_error",
            ExecutionStatus::NetworkError => "network_error",
            ExecutionStatus::Timeout => "timeout",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Oversize => "oversize",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "success" => Ok(ExecutionStatus::Success),
            "http_error" => Ok(ExecutionStatus::HttpError),
            "network_error" => Ok(ExecutionStatus::NetworkError),
            "timeout" => Ok(ExecutionStatus::Timeout),
            "cancelled" => Ok(ExecutionStatus::Cancelled),
            "oversize" => Ok(ExecutionStatus::Oversize),
            other => Err(format!("不支持的执行状态: {other}")),
        }
    }
}

/// 发送时冻结的请求快照
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSnapshot {
    #[serde(default = "default_method")]
    pub method: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub query: Vec<KeyValueRow>,

    #[serde(default)]
    pub headers: Vec<KeyValueRow>,

    #[serde(default)]
    pub body: RequestBody,

    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    #[serde(default)]
    pub environment_name: Option<String>,
}

/// 执行历史。请求快照与响应体均已脱敏、截断
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequestHistory {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub request_id: String,

    #[serde(default)]
    pub environment_name: Option<String>,

    #[serde(default)]
    pub request_snapshot: RequestSnapshot,

    pub status: ExecutionStatus,

    #[serde(default)]
    pub status_code: Option<u16>,

    #[serde(default)]
    pub response_headers: Vec<KeyValueRow>,

    #[serde(default)]
    pub response_body_preview: String,

    #[serde(default)]
    pub response_body_truncated: bool,

    #[serde(default)]
    pub duration_ms: u64,

    #[serde(default)]
    pub error_message: Option<String>,

    #[serde(default)]
    pub executed_at: i64,
}

impl Default for ApiRequestHistory {
    fn default() -> Self {
        Self {
            id: String::new(),
            request_id: String::new(),
            environment_name: None,
            request_snapshot: RequestSnapshot::default(),
            status: ExecutionStatus::Success,
            status_code: None,
            response_headers: Vec::new(),
            response_body_preview: String::new(),
            response_body_truncated: false,
            duration_ms: 0,
            error_message: None,
            executed_at: 0,
        }
    }
}

/// 执行结果。返回给前端展示，未截断
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// 本次执行 ID，取消与响应回传按该 ID 关联
    pub execution_id: String,

    pub status: ExecutionStatus,

    #[serde(default)]
    pub status_code: Option<u16>,

    #[serde(default)]
    pub response_headers: Vec<KeyValueRow>,

    #[serde(default)]
    pub body_text: String,

    /// 已接收的响应体字节数
    #[serde(default)]
    pub body_size_bytes: usize,

    #[serde(default)]
    pub content_type: Option<String>,

    /// 二进制响应仅提示类型和大小，不转换为文本
    #[serde(default)]
    pub is_binary: bool,

    /// 响应超过读取上限，已终止继续读取
    #[serde(default)]
    pub is_oversized: bool,

    #[serde(default)]
    pub duration_ms: u64,

    #[serde(default)]
    pub error_message: Option<String>,

    /// 历史写入失败时提示，不影响已收到的响应
    #[serde(default)]
    pub history_error: Option<String>,

    #[serde(default)]
    pub history_id: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// 校验并规范化 HTTP 方法
pub fn normalize_method(method: &str) -> Result<String, String> {
    let upper = method.trim().to_ascii_uppercase();
    match upper.as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS" => Ok(upper),
        other if other.is_empty() => Err("HTTP 方法不能为空".to_string()),
        other => Err(format!("不支持的 HTTP 方法: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_method_accepts_supported_methods_case_insensitively() {
        assert_eq!(normalize_method("get").expect("get"), "GET");
        assert_eq!(normalize_method(" Patch ").expect("patch"), "PATCH");
        assert_eq!(normalize_method("OPTIONS").expect("options"), "OPTIONS");
    }

    #[test]
    fn normalize_method_rejects_unsupported_methods() {
        assert!(normalize_method("TRACE").is_err());
        assert!(normalize_method("").is_err());
    }

    #[test]
    fn body_kind_round_trips_through_string() {
        for kind in [
            BodyKind::None,
            BodyKind::Json,
            BodyKind::Text,
            BodyKind::Form,
        ] {
            assert_eq!(BodyKind::parse(kind.as_str()).expect("parse"), kind);
        }
        assert!(BodyKind::parse("multipart").is_err());
    }

    #[test]
    fn execution_status_round_trips_through_string() {
        for status in [
            ExecutionStatus::Success,
            ExecutionStatus::HttpError,
            ExecutionStatus::NetworkError,
            ExecutionStatus::Timeout,
            ExecutionStatus::Cancelled,
            ExecutionStatus::Oversize,
        ] {
            assert_eq!(
                ExecutionStatus::parse(status.as_str()).expect("parse"),
                status
            );
        }
        assert!(ExecutionStatus::parse("retrying").is_err());
    }

    #[test]
    fn json_body_default_content_type_is_application_json() {
        assert_eq!(
            BodyKind::Json.default_content_type(),
            Some("application/json")
        );
        assert_eq!(BodyKind::None.default_content_type(), None);
    }
}

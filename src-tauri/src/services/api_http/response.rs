//! 响应处理：流式读取、状态判定、二进制识别、结果对象构造。
//! 不发起网络请求，仅消费 reqwest 给出的 [`Response`] 并产出 [`ExecutionResult`]。

use std::time::Instant;

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::Response;

use crate::models::api_client::{
    ExecutionResult, ExecutionStatus, KeyValueRow, MAX_RESPONSE_BYTES,
};

/// 把 reqwest 响应头转换为 `KeyValueRow` 列表。值不可文本化时用占位。
pub fn collect_headers(headers: &HeaderMap) -> Vec<KeyValueRow> {
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
pub fn is_binary_response(content_type: Option<&str>, body: &[u8]) -> bool {
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

/// 构造"已取消等待"的结果
pub fn cancelled_result(execution_id: &str, started: Instant) -> ExecutionResult {
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

/// 由 reqwest 错误构造执行结果：超时归为 Timeout，其它归为 NetworkError
pub fn error_result(
    execution_id: &str,
    started: Instant,
    error: reqwest::Error,
) -> ExecutionResult {
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

/// 流式读取响应，超过 [`MAX_RESPONSE_BYTES`] 上限即终止，不做无上限缓冲
pub async fn read_response_with_limit(
    execution_id: &str,
    response: Response,
    started: Instant,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> ExecutionResult {
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
                return cancelled_result(execution_id, started);
            }
            chunk = stream.next() => chunk,
        };

        let Some(chunk) = chunk else {
            break;
        };

        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                return ExecutionResult {
                    response_headers,
                    status_code: Some(status.as_u16()),
                    ..error_result(execution_id, started, error)
                };
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

    ExecutionResult {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

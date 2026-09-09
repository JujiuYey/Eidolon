//! HTTP 客户端与执行管线：负责构造 reqwest 客户端、维护按执行 ID 关联的取消信号、
//! 串起网络发送与流式响应读取。所有准备与响应处理逻辑分别放在
//! [`super::request_prep`] 与 [`super::response`] 中。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nanoid::nanoid;
use reqwest::{Client, Method};
use tokio::sync::watch;

use super::request_prep::{build_header_map, PreparedRequest};
use super::response::{cancelled_result, error_result, read_response_with_limit};
use crate::models::api_client::ExecutionResult;

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

    #[cfg(test)]
    fn is_inflight(&self, execution_id: &str) -> bool {
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

        Ok(read_response_with_limit(execution_id, response, started, cancel).await)
    }
}

/// 生成一个执行 ID
pub fn new_execution_id() -> String {
    format!("aexe_{}", nanoid!(12))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_marks_unknown_execution_as_not_inflight() {
        let registry = ExecutionRegistry::new();
        assert!(!registry.is_inflight("missing"));
        assert!(!registry.cancel("missing").expect("cancel should not fail"));
    }
}

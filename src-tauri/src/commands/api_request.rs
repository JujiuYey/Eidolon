use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_history::ApiHistoryRepository;
use crate::db::repositories::api_request_repo::ApiRequestRepository;
use crate::models::api_client::{ApiRequestHistory, ExecutionResult, RequestSnapshot};
use crate::services::api_http::{new_execution_id, prepare_request, ApiHttpClient};

/// 发送一次请求。快照由前端冻结，发送不会写回请求定义。
#[tauri::command]
pub async fn send_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    http_client: tauri::State<'_, ApiHttpClient>,
    request_id: String,
    environment_id: Option<String>,
    snapshot: RequestSnapshot,
    execution_id: Option<String>,
) -> Result<ExecutionResult, String> {
    let repository = ApiRequestRepository::new(&database);
    let environment = repository.resolve_environment(&request_id, environment_id.as_deref())?;

    let prepared = prepare_request(&snapshot, environment.as_ref()).map_err(String::from)?;

    let execution_id = execution_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(new_execution_id);

    let mut result = http_client.send(&execution_id, &prepared).await?;

    let environment_name = environment
        .as_ref()
        .map(|environment| environment.name.clone());

    // 历史写入失败不能吞掉已经收到的响应
    let history = ApiHistoryRepository::new(&database).append(&ApiRequestHistory {
        request_id: request_id.clone(),
        environment_name: environment_name.clone(),
        request_snapshot: RequestSnapshot {
            environment_name,
            ..snapshot
        },
        status: result.status,
        status_code: result.status_code,
        response_headers: result.response_headers.clone(),
        response_body_preview: result.body_text.clone(),
        response_body_truncated: result.is_oversized,
        duration_ms: result.duration_ms,
        error_message: result.error_message.clone(),
        ..Default::default()
    });

    match history {
        Ok(saved) => result.history_id = Some(saved.id),
        Err(error) => result.history_error = Some(format!("响应已收到，但历史未保存: {error}")),
    }

    Ok(result)
}

/// 取消等待。服务端可能已经执行，不显示为“已回滚”。
#[tauri::command]
pub fn cancel_api_request(
    http_client: tauri::State<'_, ApiHttpClient>,
    execution_id: String,
) -> Result<bool, String> {
    http_client.registry().cancel(&execution_id)
}

/// 校验但不发送：用于在 UI 上提前提示缺失变量和不合法的 JSON
#[tauri::command]
pub fn preview_api_request(
    database: tauri::State<'_, ApiClientDatabase>,
    request_id: String,
    environment_id: Option<String>,
    snapshot: RequestSnapshot,
) -> Result<PreparedRequestPreview, String> {
    let repository = ApiRequestRepository::new(&database);
    let environment = repository.resolve_environment(&request_id, environment_id.as_deref())?;
    let prepared = prepare_request(&snapshot, environment.as_ref()).map_err(String::from)?;

    Ok(PreparedRequestPreview {
        method: prepared.method,
        url: prepared.url,
        headers: prepared.headers,
        body_size_bytes: prepared.body.as_ref().map(|body| body.len()).unwrap_or(0),
        environment_name: environment.map(|environment| environment.name),
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreparedRequestPreview {
    pub method: String,
    pub url: String,
    pub headers: Vec<crate::models::api_client::KeyValueRow>,
    pub body_size_bytes: usize,
    pub environment_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::net::TcpListener;

    use super::*;
    use crate::models::api_client::{
        ApiEnvironment, BodyKind, ExecutionStatus, KeyValueRow, RequestBody, MAX_RESPONSE_BYTES,
    };
    use crate::services::api_http::PrepareError;

    fn is_received_response(status: ExecutionStatus) -> bool {
        matches!(
            status,
            ExecutionStatus::Success | ExecutionStatus::HttpError | ExecutionStatus::Oversize
        )
    }

    /// 极简 HTTP 测试服务器：按路径返回固定响应，避免引入额外框架
    async fn spawn_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("local addr");

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };

                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};

                    let mut buffer = vec![0u8; 16 * 1024];
                    let read = match stream.read(&mut buffer).await {
                        Ok(read) => read,
                        Err(_) => return,
                    };
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let first_line = request.lines().next().unwrap_or_default().to_string();

                    let response: Vec<u8> = if first_line.contains("/echo") {
                        let body = request
                            .split_once("\r\n\r\n")
                            .map(|(_, body)| body.to_string())
                            .unwrap_or_default();
                        let method = first_line.split_whitespace().next().unwrap_or("GET");
                        let payload = format!(
                            "{{\"method\":\"{method}\",\"body\":{}}}",
                            if body.trim().is_empty() {
                                "null".to_string()
                            } else {
                                format!("{body}")
                            }
                        );
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .into_bytes()
                    } else if first_line.contains("/not-found") {
                        let payload = "{\"error\":\"missing\"}";
                        format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .into_bytes()
                    } else if first_line.contains("/server-error") {
                        let payload = "boom";
                        format!(
                            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .into_bytes()
                    } else if first_line.contains("/redirect") {
                        "HTTP/1.1 302 Found\r\nLocation: /echo\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_string()
                            .into_bytes()
                    } else if first_line.contains("/binary") {
                        let body: Vec<u8> = vec![0x89, 0x50, 0x4e, 0x47, 0x00, 0x01, 0x02];
                        let mut head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .into_bytes();
                        head.extend_from_slice(&body);
                        head
                    } else if first_line.contains("/huge") {
                        let size = MAX_RESPONSE_BYTES + 64 * 1024;
                        let mut head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
                        )
                        .into_bytes();
                        head.extend(std::iter::repeat(b'a').take(size));
                        head
                    } else if first_line.contains("/slow") {
                        tokio::time::sleep(Duration::from_millis(1_500)).await;
                        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                            .to_string()
                            .into_bytes()
                    } else {
                        let payload = "{\"ok\":true}";
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .into_bytes()
                    };

                    let _ = stream.write_all(&response).await;
                    let _ = stream.flush().await;
                });
            }
        });

        address
    }

    fn snapshot(url: &str) -> RequestSnapshot {
        RequestSnapshot {
            method: "GET".to_string(),
            url: url.to_string(),
            query: Vec::new(),
            headers: Vec::new(),
            body: RequestBody::default(),
            timeout_ms: 5_000,
            environment_name: None,
        }
    }

    async fn send(snapshot: &RequestSnapshot) -> ExecutionResult {
        let client = ApiHttpClient::new().expect("client");
        let prepared = prepare_request(snapshot, None).expect("prepare");
        client
            .send(&new_execution_id(), &prepared)
            .await
            .expect("send should answer")
    }

    #[tokio::test]
    async fn get_request_returns_success_with_body_and_headers() {
        let address = spawn_server().await;
        let result = send(&snapshot(&format!("http://{address}/echo"))).await;

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.status_code, Some(200));
        assert!(
            result.body_text.contains("\"method\":\"GET\""),
            "got {}",
            result.body_text
        );
        assert!(result.body_size_bytes > 0);
        assert_eq!(result.content_type.as_deref(), Some("application/json"));
        assert!(result
            .response_headers
            .iter()
            .any(|row| row.key.eq_ignore_ascii_case("content-type")));
        assert!(is_received_response(result.status));
    }

    #[tokio::test]
    async fn post_json_body_reaches_the_server() {
        let address = spawn_server().await;
        let result = send(&RequestSnapshot {
            method: "POST".to_string(),
            body: RequestBody {
                kind: BodyKind::Json,
                text: "{\"items\":3}".to_string(),
                form: Vec::new(),
            },
            ..snapshot(&format!("http://{address}/echo"))
        })
        .await;

        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.body_text.contains("\"method\":\"POST\""));
        assert!(
            result.body_text.contains("\"items\":3"),
            "got {}",
            result.body_text
        );
    }

    #[tokio::test]
    async fn http_4xx_and_5xx_are_received_responses() {
        let address = spawn_server().await;

        let not_found = send(&snapshot(&format!("http://{address}/not-found"))).await;
        assert_eq!(not_found.status, ExecutionStatus::HttpError);
        assert_eq!(not_found.status_code, Some(404));
        assert!(not_found.body_text.contains("missing"));

        let server_error = send(&snapshot(&format!("http://{address}/server-error"))).await;
        assert_eq!(server_error.status, ExecutionStatus::HttpError);
        assert_eq!(server_error.status_code, Some(500));
        assert_eq!(server_error.body_text, "boom");
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let address = spawn_server().await;
        let result = send(&snapshot(&format!("http://{address}/redirect"))).await;

        assert_eq!(result.status_code, Some(302));
        assert!(result
            .response_headers
            .iter()
            .any(|row| row.key.eq_ignore_ascii_case("location")));
    }

    #[tokio::test]
    async fn binary_response_is_reported_by_type_and_size_only() {
        let address = spawn_server().await;
        let result = send(&snapshot(&format!("http://{address}/binary"))).await;

        assert!(result.is_binary);
        assert!(
            result.body_text.is_empty(),
            "binary body must not be converted"
        );
        assert_eq!(result.body_size_bytes, 7);
        assert_eq!(result.content_type.as_deref(), Some("image/png"));
    }

    #[tokio::test]
    async fn response_beyond_limit_stops_reading_and_is_flagged() {
        let address = spawn_server().await;
        let result = send(&snapshot(&format!("http://{address}/huge"))).await;

        assert!(result.is_oversized);
        assert_eq!(result.status, ExecutionStatus::Oversize);
        assert_eq!(result.body_size_bytes, MAX_RESPONSE_BYTES);
        assert!(result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("响应超过上限"));
    }

    #[tokio::test]
    async fn timeout_is_an_execution_error() {
        let address = spawn_server().await;
        let result = send(&RequestSnapshot {
            timeout_ms: 200,
            ..snapshot(&format!("http://{address}/slow"))
        })
        .await;

        assert_eq!(result.status, ExecutionStatus::Timeout);
        assert_eq!(result.status_code, None);
        assert!(!is_received_response(result.status));
    }

    #[tokio::test]
    async fn connection_failure_is_an_execution_error() {
        // 关闭的端口：连接直接失败
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("addr");
        drop(listener);

        let result = send(&snapshot(&format!("http://{address}/echo"))).await;

        assert_eq!(result.status, ExecutionStatus::NetworkError);
        assert_eq!(result.status_code, None);
        assert!(result.error_message.is_some());
    }

    #[tokio::test]
    async fn cancel_stops_waiting_and_reports_cancelled() {
        let address = spawn_server().await;
        let client = std::sync::Arc::new(ApiHttpClient::new().expect("client"));
        let prepared = prepare_request(
            &RequestSnapshot {
                timeout_ms: 10_000,
                ..snapshot(&format!("http://{address}/slow"))
            },
            None,
        )
        .expect("prepare");

        let execution_id = new_execution_id();
        let registry = client.registry();
        let send_client = std::sync::Arc::clone(&client);
        let send_id = execution_id.clone();
        let handle = tokio::spawn(async move { send_client.send(&send_id, &prepared).await });

        // 等到执行登记完成后再取消
        for _ in 0..100 {
            if registry.is_inflight(&execution_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(registry.cancel(&execution_id).expect("cancel"));

        let result = handle.await.expect("join").expect("send should answer");
        assert_eq!(result.status, ExecutionStatus::Cancelled);
        assert!(result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("已取消等待"));
        assert!(
            !registry.is_inflight(&execution_id),
            "registry must be cleaned up"
        );
    }

    #[tokio::test]
    async fn duplicate_query_and_headers_are_sent_in_order() {
        let address = spawn_server().await;
        let prepared = prepare_request(
            &RequestSnapshot {
                query: vec![KeyValueRow::new("tag", "a"), KeyValueRow::new("tag", "b")],
                headers: vec![
                    KeyValueRow::new("X-Multi", "one"),
                    KeyValueRow::new("X-Multi", "two"),
                ],
                ..snapshot(&format!("http://{address}/echo"))
            },
            None,
        )
        .expect("prepare");

        assert!(
            prepared.url.ends_with("?tag=a&tag=b"),
            "got {}",
            prepared.url
        );

        let client = ApiHttpClient::new().expect("client");
        let result = client
            .send(&new_execution_id(), &prepared)
            .await
            .expect("send");
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[test]
    fn missing_variables_are_reported_before_any_request_is_made() {
        let environment = ApiEnvironment {
            id: "env1".to_string(),
            project_id: "p1".to_string(),
            name: "测试环境".to_string(),
            base_url: "http://127.0.0.1:1".to_string(),
            variables: Vec::new(),
            created_at: 0,
            updated_at: 0,
        };

        let error = prepare_request(
            &snapshot("{{base_url}}/orders/{{order_id}}"),
            Some(&environment),
        )
        .expect_err("missing variables block sending");

        assert!(
            matches!(error, PrepareError::MissingVariables(ref names) if names == &vec!["order_id".to_string()])
        );
    }
}

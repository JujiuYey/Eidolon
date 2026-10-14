//! 模型请求失败的错误翻译。
//!
//! rig 的 `CompletionError` 直接 Display 出来是
//! `HttpError: Http client error: error sending request for url (...)` 这种
//! 底层报文——对用户毫无意义。这里把它翻成能指导下一步动作的中文，
//! 并按故障类型给出不同的处置建议（重试 / 查 Key / 查代理 / 等一会）。

use rig::completion::CompletionError;
use rig::http_client;

/// 状态码 → 该怎么跟用户解释
fn describe_status(status: u16, body: &str) -> String {
    let hint = match status {
        401 | 403 => "请到「应用设置」检查该模型的 API Key 是否正确或已过期",
        429 => "请求过于频繁或额度已用尽，稍后重试或检查账户余额",
        500..=599 => "模型服务自身出错了，这是对方的问题，过一会儿再试",
        400..=499 => "请求被模型服务拒绝，多半是模型名或参数不对",
        _ => return format!("模型服务返回了异常状态（{status}）"),
    };
    match body.trim() {
        "" => format!("模型服务拒绝了请求（HTTP {status}）：{hint}"),
        detail => format!("模型服务拒绝了请求（HTTP {status}）：{hint}。服务返回：{detail}"),
    }
}

/// 把 rig 的底层 HTTP 错误翻成人话。
fn describe_http(error: &http_client::Error) -> String {
    match error {
        // 真正发请求失败的那一类，超时/连不上/DNS/TLS 都在里面
        http_client::Error::Instance(inner) => {
            let reqwest_error = inner.downcast_ref::<reqwest::Error>();
            if let Some(reqwest_error) = reqwest_error {
                if reqwest_error.is_timeout() {
                    return "模型请求超时，网络太慢或服务无响应。可以稍后重试，或换个更快的模型。"
                        .to_string();
                }
                if reqwest_error.is_connect() {
                    return "连不上模型服务。请检查网络、系统代理，以及该模型服务的 API 地址是否可达。"
                        .to_string();
                }
                if reqwest_error.is_redirect() {
                    return "模型服务地址跳转异常，API 地址可能填错了。".to_string();
                }
                if reqwest_error.is_body() {
                    return "读取模型响应时连接中断，可能是网络波动，可以重试。".to_string();
                }
            }
            // downcast 不到时保留底层信息，至少比原始报文短
            format!("无法访问模型服务：{inner}")
        }
        http_client::Error::InvalidStatusCode(status) => describe_status(status.as_u16(), ""),
        http_client::Error::InvalidStatusCodeWithMessage(status, message) => {
            describe_status(status.as_u16(), &message)
        }
        http_client::Error::Protocol(inner) => format!("模型服务通信异常：{inner}"),
        http_client::Error::InvalidContentType(value) => {
            format!("模型服务返回的不是预期内容类型：{value:?}")
        }
        http_client::Error::InvalidHeaderValue(inner) => format!("模型服务响应头异常：{inner}"),
        http_client::Error::NoHeaders => "读取模型响应时缺少响应头。".to_string(),
        http_client::Error::StreamEnded => "模型响应流意外中断，可以重试。".to_string(),
    }
}

/// 模型请求失败的统一文案
pub fn describe_completion_error(error: &CompletionError) -> String {
    match error {
        CompletionError::HttpError(inner) => describe_http(inner),
        CompletionError::JsonError(inner) => {
            format!("模型返回的内容不是合法 JSON，无法解析：{inner}")
        }
        CompletionError::UrlError(inner) => {
            format!("模型服务地址配置有误：{inner}")
        }
        CompletionError::RequestError(inner) => {
            format!("构造模型请求失败：{inner}")
        }
        CompletionError::ResponseError(message) => format!("模型响应异常：{message}"),
        CompletionError::ProviderError(message) => format!("模型服务报错：{message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_auth_and_rate_limit_to_actionable_hints() {
        let error = CompletionError::HttpError(http_client::Error::InvalidStatusCodeWithMessage(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key".to_string(),
        ));
        let text = describe_completion_error(&error);
        assert!(text.contains("API Key"), "got {text}");
        assert!(!text.contains("HttpError"), "不应泄漏底层类型名：{text}");

        let limited = CompletionError::HttpError(http_client::Error::InvalidStatusCodeWithMessage(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            String::new(),
        ));
        assert!(describe_completion_error(&limited).contains("额度"));
    }

    #[test]
    fn maps_server_fault_without_blaming_local_config() {
        let error = CompletionError::HttpError(http_client::Error::InvalidStatusCode(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        ));
        let text = describe_completion_error(&error);
        assert!(text.contains("模型服务自身"), "got {text}");
    }

    #[test]
    fn never_leaks_rig_variant_names() {
        let samples = [
            CompletionError::JsonError(serde_json::from_str::<u8>("nope").unwrap_err()),
            CompletionError::ResponseError("no choices".to_string()),
            CompletionError::ProviderError("model overloaded".to_string()),
        ];
        for sample in samples {
            let text = describe_completion_error(&sample);
            assert!(!text.contains("JsonError"), "{text}");
            assert!(!text.contains("ResponseError"), "{text}");
            assert!(!text.contains("ProviderError"), "{text}");
        }
    }
}

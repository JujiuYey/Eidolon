use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel};
use rig::providers::openai::Client;
use serde::{Deserialize, Serialize};

use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::api_client::BODY_GENERATION_MODEL_KEY;
use crate::models::default_model::DefaultModelSetting;
use crate::models::model_config::ProviderSetting;

const SYSTEM_PROMPT: &str = "你是接口请求体生成助手。根据用户提供的生成要求和参考内容，输出一个可直接作为 HTTP 请求体使用的 JSON。\
只输出 JSON 本身，不要输出解释文字，不要使用 Markdown 代码块。JSON 顶层可以是对象、数组或其他合法 JSON 值。";

/// 生成请求。上下文只包含用户明确提供的内容与有限的请求元数据。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GenerateBodyInput {
    /// 生成要求
    #[serde(default)]
    pub prompt: String,

    /// 参考内容：字段说明、文档片段或 JSON 示例
    #[serde(default)]
    pub reference: String,

    /// 当前 Body，仅在用户勾选带入时提供
    #[serde(default)]
    pub current_body: Option<String>,

    /// 请求名称，作为辅助上下文
    #[serde(default)]
    pub request_name: String,

    /// HTTP 方法，作为辅助上下文
    #[serde(default)]
    pub method: String,

    /// 未展开变量的 URL 路径，作为辅助上下文
    #[serde(default)]
    pub url: String,
}

/// 候选结果。校验结果随内容一起返回，由前端决定是否允许应用。
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedBodyCandidate {
    pub content: String,
    pub is_json_valid: bool,
    pub json_error: Option<String>,
    pub model_label: String,
}

#[derive(Debug, Clone, PartialEq)]
struct GenerationTarget {
    model_id: String,
    api_key: String,
    base_url: String,
    provider_id: String,
    temperature: Option<f64>,
    max_tokens: Option<u64>,
}

impl GenerationTarget {
    fn label(&self) -> String {
        format!("{} / {}", self.provider_id, self.model_id)
    }
}

/// 生成候选请求体。不发送业务接口，也不写入请求定义。
#[tauri::command]
pub async fn generate_api_request_body(
    store: tauri::State<'_, LocalJsonStore>,
    input: GenerateBodyInput,
) -> Result<GeneratedBodyCandidate, String> {
    let target = {
        let default_repo = DefaultModelSettingRepository::new(&store);
        let provider_repo = ProviderSettingRepository::new(&store);
        resolve_generation_target(&default_repo.list()?, &provider_repo.list()?)?
    };

    let user_prompt = build_generation_prompt(&input)?;

    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let mut request = client
        .completion_model(&target.model_id)
        .completion_request(rig::completion::Message::user(user_prompt))
        .preamble(SYSTEM_PROMPT.to_string());

    if let Some(temperature) = target.temperature {
        request = request.temperature(temperature);
    }

    if let Some(max_tokens) = target.max_tokens {
        request = request.max_tokens(max_tokens);
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("模型请求失败: {error}"))?;

    let raw = extract_text(response.choice).ok_or_else(|| "模型未返回文本内容".to_string())?;
    let content = strip_code_fence(&raw);

    Ok(build_candidate(&content, &target.label()))
}

/// 请求体生成使用独立默认模型键，不复用也不改写聊天默认模型
fn resolve_generation_target(
    default_settings: &[DefaultModelSetting],
    provider_settings: &[ProviderSetting],
) -> Result<GenerationTarget, String> {
    let default_setting = default_settings
        .iter()
        .find(|setting| setting.key == BODY_GENERATION_MODEL_KEY)
        .ok_or_else(|| {
            "请先在“默认模型”中配置请求体生成模型，配置前仍可手动编辑和发送请求".to_string()
        })?;

    let provider_setting = provider_settings
        .iter()
        .find(|setting| setting.provider_id == default_setting.provider_id)
        .ok_or_else(|| format!("未找到 {} 的模型服务配置", default_setting.provider_id))?;

    if !provider_setting.enabled {
        return Err(format!(
            "{} 已被禁用，请先启用后再生成请求体",
            provider_setting.provider_id
        ));
    }

    if provider_setting.api_key.trim().is_empty() {
        return Err(format!(
            "{} 的 API Key 不能为空",
            provider_setting.provider_id
        ));
    }

    if provider_setting.base_url.trim().is_empty() {
        return Err(format!(
            "{} 的 Base URL 不能为空",
            provider_setting.provider_id
        ));
    }

    Ok(GenerationTarget {
        model_id: default_setting.model_id.clone(),
        api_key: provider_setting.api_key.clone(),
        base_url: provider_setting.base_url.clone(),
        provider_id: provider_setting.provider_id.clone(),
        temperature: parse_optional_f64(&default_setting.temperature, "temperature")?,
        max_tokens: parse_optional_u64(&default_setting.max_tokens, "max_tokens")?,
    })
}

/// 组装上下文。不带入 Headers、环境变量值和响应历史。
fn build_generation_prompt(input: &GenerateBodyInput) -> Result<String, String> {
    let prompt = input.prompt.trim();
    let reference = input.reference.trim();

    if prompt.is_empty() && reference.is_empty() {
        return Err("请先填写生成要求或参考内容".to_string());
    }

    let mut sections: Vec<String> = Vec::new();

    let name = input.request_name.trim();
    let method = input.method.trim();
    let url = input.url.trim();

    if !name.is_empty() || !method.is_empty() || !url.is_empty() {
        let mut meta = Vec::new();
        if !name.is_empty() {
            meta.push(format!("名称: {name}"));
        }
        if !method.is_empty() {
            meta.push(format!("方法: {method}"));
        }
        if !url.is_empty() {
            meta.push(format!("地址: {url}"));
        }
        sections.push(format!("## 接口信息\n{}", meta.join("\n")));
    }

    if !prompt.is_empty() {
        sections.push(format!("## 生成要求\n{prompt}"));
    }

    if !reference.is_empty() {
        sections.push(format!("## 参考内容\n{reference}"));
    }

    if let Some(current_body) = input
        .current_body
        .as_deref()
        .map(str::trim)
        .filter(|body| !body.is_empty())
    {
        sections.push(format!("## 当前请求体\n{current_body}"));
    }

    sections.push("## 输出要求\n只输出 JSON 本身。".to_string());

    Ok(sections.join("\n\n"))
}

/// JSON 语法校验。对象、数组和标量都算合法。
fn build_candidate(content: &str, model_label: &str) -> GeneratedBodyCandidate {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(_) => GeneratedBodyCandidate {
            content: content.to_string(),
            is_json_valid: true,
            json_error: None,
            model_label: model_label.to_string(),
        },
        Err(error) => GeneratedBodyCandidate {
            content: content.to_string(),
            is_json_valid: false,
            json_error: Some(format!("JSON 语法错误: {error}")),
            model_label: model_label.to_string(),
        },
    }
}

/// 模型有时会包裹 Markdown 代码块，这里只剥掉围栏，不改内容
fn strip_code_fence(raw: &str) -> String {
    let trimmed = raw.trim();

    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let without_open = trimmed.trim_start_matches('`');
    let body = match without_open.find('\n') {
        Some(index) => &without_open[index + 1..],
        None => return trimmed.to_string(),
    };

    body.trim_end().trim_end_matches('`').trim_end().to_string()
}

fn extract_text(choice: rig::OneOrMany<AssistantContent>) -> Option<String> {
    let text = choice
        .into_iter()
        .filter_map(|content| match content {
            AssistantContent::Text(text) => Some(text.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_optional_f64(value: &str, field: &str) -> Result<Option<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("{field} 格式不正确"))
}

fn parse_optional_u64(value: &str, field: &str) -> Result<Option<u64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("{field} 格式不正确"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(provider_id: &str) -> ProviderSetting {
        ProviderSetting {
            provider_id: provider_id.to_string(),
            enabled: true,
            api_key: "sk-test".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
        }
    }

    fn default_setting(key: &str) -> DefaultModelSetting {
        DefaultModelSetting {
            key: key.to_string(),
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
            temperature: "0.2".to_string(),
            top_p: String::new(),
            max_tokens: "2048".to_string(),
            presence_penalty: String::new(),
            frequency_penalty: String::new(),
        }
    }

    #[test]
    fn generation_uses_its_own_default_model_key_not_the_chat_one() {
        let error =
            resolve_generation_target(&[default_setting("assistant")], &[provider("deepseek")])
                .expect_err("chat default model must not be reused");
        assert!(error.contains("请求体生成模型"), "got {error}");

        let target = resolve_generation_target(
            &[
                default_setting("assistant"),
                default_setting(BODY_GENERATION_MODEL_KEY),
            ],
            &[provider("deepseek")],
        )
        .expect("dedicated key should resolve");

        assert_eq!(target.model_id, "deepseek-chat");
        assert_eq!(target.temperature, Some(0.2));
        assert_eq!(target.max_tokens, Some(2048));
        assert_eq!(target.label(), "deepseek / deepseek-chat");
    }

    #[test]
    fn generation_requires_an_enabled_provider_with_credentials() {
        let missing = resolve_generation_target(&[default_setting(BODY_GENERATION_MODEL_KEY)], &[])
            .expect_err("provider setting is required");
        assert!(missing.contains("模型服务配置"), "got {missing}");

        let disabled = resolve_generation_target(
            &[default_setting(BODY_GENERATION_MODEL_KEY)],
            &[ProviderSetting {
                enabled: false,
                ..provider("deepseek")
            }],
        )
        .expect_err("disabled provider must be rejected");
        assert!(disabled.contains("已被禁用"), "got {disabled}");

        let no_key = resolve_generation_target(
            &[default_setting(BODY_GENERATION_MODEL_KEY)],
            &[ProviderSetting {
                api_key: "  ".to_string(),
                ..provider("deepseek")
            }],
        )
        .expect_err("empty api key must be rejected");
        assert!(no_key.contains("API Key"), "got {no_key}");
    }

    #[test]
    fn context_excludes_headers_and_only_includes_opted_in_body() {
        let without_body = build_generation_prompt(&GenerateBodyInput {
            prompt: "生成一个订单，包含三件商品".to_string(),
            reference: "items 为数组".to_string(),
            current_body: None,
            request_name: "新增订单".to_string(),
            method: "POST".to_string(),
            url: "{{base_url}}/orders".to_string(),
        })
        .expect("prompt should build");

        assert!(without_body.contains("生成一个订单，包含三件商品"));
        assert!(without_body.contains("items 为数组"));
        assert!(without_body.contains("新增订单"));
        assert!(
            without_body.contains("{{base_url}}/orders"),
            "unexpanded url is context"
        );
        assert!(!without_body.contains("当前请求体"), "body must be opt-in");
        assert!(!without_body.to_lowercase().contains("authorization"));

        let with_body = build_generation_prompt(&GenerateBodyInput {
            prompt: "补齐字段".to_string(),
            current_body: Some("{\"userId\":1}".to_string()),
            ..Default::default()
        })
        .expect("prompt should build");

        assert!(with_body.contains("## 当前请求体"));
        assert!(with_body.contains("{\"userId\":1}"));
    }

    #[test]
    fn empty_prompt_and_reference_are_rejected() {
        let error = build_generation_prompt(&GenerateBodyInput {
            current_body: Some("{}".to_string()),
            ..Default::default()
        })
        .expect_err("nothing to generate from");

        assert!(error.contains("生成要求"), "got {error}");
    }

    #[test]
    fn candidate_accepts_objects_arrays_and_scalars() {
        for content in ["{\"a\":1}", "[1,2,3]", "\"text\"", "42", "true", "null"] {
            let candidate = build_candidate(content, "deepseek / deepseek-chat");
            assert!(candidate.is_json_valid, "{content} should be valid JSON");
            assert!(candidate.json_error.is_none());
            assert_eq!(candidate.content, content);
        }
    }

    #[test]
    fn candidate_reports_invalid_json_without_dropping_content() {
        let candidate = build_candidate("{\"a\": }", "deepseek / deepseek-chat");

        assert!(!candidate.is_json_valid);
        assert!(candidate
            .json_error
            .as_deref()
            .unwrap_or_default()
            .contains("JSON 语法错误"));
        assert_eq!(
            candidate.content, "{\"a\": }",
            "content must be preserved for editing"
        );
    }

    #[test]
    fn markdown_fences_are_stripped_before_validation() {
        let fenced = "```json\n{\"a\": 1}\n```";
        assert_eq!(strip_code_fence(fenced), "{\"a\": 1}");
        assert!(build_candidate(&strip_code_fence(fenced), "m").is_json_valid);

        let bare = "{\"a\": 1}";
        assert_eq!(strip_code_fence(bare), bare);
    }
}

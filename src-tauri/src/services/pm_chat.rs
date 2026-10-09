use std::path::Path;

use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionModel, Message as RigMessage};
use rig::providers::openai::Client;

use crate::commands::api_generate::{self, GenerationTarget};
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::default_model::DefaultModelSettingRepository;
use crate::db::repositories::model_config::ProviderSettingRepository;
use crate::models::pm::PmMessage;
use crate::services::pm_skills::{self, PmSkill};

pub const PM_CHAT_MODEL_KEY: &str = "pm_chat";

/// 单轮回答最多注入的框架全文数（防止上下文被撑爆）
const MAX_SKILLS_PER_TURN: usize = 3;
/// 索引中单个框架描述的截断长度（字符）
const INDEX_DESCRIPTION_CHARS: usize = 200;
/// 路由请求可看到的最近消息条数
const ROUTER_CONTEXT_MESSAGES: usize = 6;
/// 路由上下文中单条消息的截断长度（字符）
const ROUTER_MESSAGE_CHARS: usize = 200;

const PM_PERSONA_PROMPT: &str = "你是一位资深产品经理，也是用户的产品顾问——用户会带着任何产品、需求、设计方面的问题来找你对话。\n\n\
背景与风格：\n\
- 十年以上 toB 与 toC 产品经验，兼通 UX 设计与 AI 产品设计，熟悉主流的产品方法论与设计框架。\n\
- 默认使用中文，语气专业但口语自然，像同事讨论，不像汇报腔。\n\
- 面对模糊的问题，先弄清三件事再给结论：目标用户是谁、使用场景是什么、怎样算成功。信息不足时直接追问，一次最多问两三个问题，不要连环审问。\n\
- 给建议时先给结论，再给理由和落地步骤，能举具体例子就举例子。\n\
- 诚实：没有依据就明说，不编造数据；对有风险的判断明确标注风险。\n\
\n\
方法论使用规则：\n\
- 你随身带有一套方法论框架库（索引见下）。回答时自然地运用合适的框架，让框架服务于问题，不堆砌术语、不为用框架而用框架。\n\
- 当某个框架的完整方法文本已在下方提供时，严格按照该框架的步骤和输出格式执行。\n\
- 没有合适框架时，就凭专业判断直接回答。\n\
- 不要在回答里罗列\"我用了什么框架\"。\n\
\n\
输出格式：\n\
- 长度与问题匹配：小问题短回答，复杂问题分节，多用列表和小标题。";

const ROUTER_PREAMBLE: &str = "你是一个方法论框架路由器。根据对话内容，从框架索引中选出回答最后一条用户消息时最值得参考的框架。\
规则：最多选 3 个；与任何框架都无关时输出 []；只输出一个 JSON 字符串数组，不要输出任何其他文字。\
示例：[\"ux-heuristics-review\",\"craft\"]";

/// 一轮 PM 回答的结果
#[derive(Debug, Clone)]
pub struct PmTurnResult {
    pub content: String,
    /// 本轮参考的框架 slug，由路由阶段决定（路由失败时为空）
    pub skills_used: Vec<String>,
    pub model_label: String,
}

/// 执行一轮 PM 对话：解析模型 → 加载技能库 → 路由选框架 → 主回答。
/// 只做模型调用，不落库；由命令层在成功后一并写入用户与助手消息。
pub async fn run_pm_turn(
    store: &LocalJsonStore,
    builtin_skills_dir: &Path,
    skills_override_dir: Option<&str>,
    history: &[PmMessage],
    content: &str,
) -> Result<PmTurnResult, String> {
    let content = content.trim();
    if content.is_empty() {
        return Err("消息内容不能为空".to_string());
    }

    let target = {
        let default_repo = DefaultModelSettingRepository::new(store);
        let provider_repo = ProviderSettingRepository::new(store);
        api_generate::resolve_generation_target(
            &default_repo.list()?,
            &provider_repo.list()?,
            PM_CHAT_MODEL_KEY,
            "请先在「应用设置 → 默认模型」中配置产品经理模型，配置后即可对话",
        )?
    };

    let skills = pm_skills::load_skill_library(builtin_skills_dir, skills_override_dir);
    let index = build_skill_index(&skills);

    let skills_used = select_skills(&target, &index, &skills, history, content).await;

    let mut preamble = format!(
        "{PM_PERSONA_PROMPT}\n\n## 可用方法论框架索引\n{index}"
    );

    if !skills_used.is_empty() {
        let full_texts = skills_used
            .iter()
            .filter_map(|slug| skills.iter().find(|skill| &skill.slug == slug))
            .map(|skill| {
                format!(
                    "<skill slug=\"{}\" name=\"{}\">\n{}\n</skill>",
                    skill.slug, skill.name, skill.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        preamble.push_str(&format!(
            "\n\n## 本轮已加载的框架全文（严格按其方法执行）\n{full_texts}"
        ));
    }

    let client = Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
        .map_err(|error| format!("创建客户端失败: {error}"))?
        .completions_api();

    let mut request = client
        .completion_model(&target.model_id)
        .completion_request(RigMessage::user(content))
        .preamble(preamble);

    for message in history {
        let rig_message = match message.role.as_str() {
            "user" => RigMessage::user(&message.content),
            "assistant" => RigMessage::assistant(&message.content),
            _ => continue,
        };
        request = request.message(rig_message);
    }

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

    let answer = extract_text(response.choice)
        .ok_or_else(|| "模型未返回可展示的文本内容".to_string())?;

    Ok(PmTurnResult {
        content: answer,
        skills_used,
        model_label: target.label(),
    })
}

/// 路由阶段：让模型从索引中挑 0-3 个框架。尽力而为——
/// 任何失败（请求错误、输出不是合法 JSON、出现未知 slug）都退化为空选，
/// 主回答仍可依靠索引中的框架摘要进行。
async fn select_skills(
    target: &GenerationTarget,
    index: &str,
    skills: &[PmSkill],
    history: &[PmMessage],
    content: &str,
) -> Vec<String> {
    let recent: Vec<String> = history
        .iter()
        .rev()
        .take(ROUTER_CONTEXT_MESSAGES)
        .rev()
        .map(|message| {
            let speaker = if message.role == "user" { "用户" } else { "产品经理" };
            truncate_chars(&message.content, ROUTER_MESSAGE_CHARS)
                .map(|truncated| format!("{speaker}: {truncated}"))
                .unwrap_or_default()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let context = if recent.is_empty() {
        "（新对话，暂无上文）".to_string()
    } else {
        recent.join("\n")
    };

    let prompt = format!(
        "框架索引：\n{index}\n\n最近对话：\n{context}\n\n用户最新消息：\n{content}"
    );

    let client = match Client::builder()
        .api_key(target.api_key.trim())
        .base_url(target.base_url.trim())
        .build()
    {
        Ok(client) => client.completions_api(),
        Err(_) => return Vec::new(),
    };

    let response = client
        .completion_model(&target.model_id)
        .completion_request(RigMessage::user(prompt))
        .preamble(ROUTER_PREAMBLE.to_string())
        .temperature(0.0)
        .send()
        .await;

    let Ok(response) = response else {
        return Vec::new();
    };

    let Some(raw) = extract_text(response.choice) else {
        return Vec::new();
    };

    parse_skill_selection(&raw, skills)
}

/// 解析路由输出：截取第一个 '[' 到最后一个 ']' 之间的内容当作 JSON 数组，
/// 过滤未知 slug、去重、截断到上限
fn parse_skill_selection(raw: &str, skills: &[PmSkill]) -> Vec<String> {
    let Some(start) = raw.find('[') else {
        return Vec::new();
    };
    let Some(end) = raw.rfind(']') else {
        return Vec::new();
    };
    if end <= start {
        return Vec::new();
    }

    let Ok(parsed) = serde_json::from_str::<Vec<String>>(&raw[start..=end]) else {
        return Vec::new();
    };

    let mut selected: Vec<String> = Vec::new();
    for slug in parsed {
        if skills.iter().any(|skill| skill.slug == slug) && !selected.contains(&slug) {
            selected.push(slug);
            if selected.len() >= MAX_SKILLS_PER_TURN {
                break;
            }
        }
    }

    selected
}

/// 常驻索引：每行 `slug — 描述（截断）`。描述缺失的框架只列 slug
fn build_skill_index(skills: &[PmSkill]) -> String {
    if skills.is_empty() {
        return "（当前没有可用的方法论框架，凭专业判断直接回答）".to_string();
    }

    skills
        .iter()
        .map(|skill| {
            let description = truncate_chars(&skill.description, INDEX_DESCRIPTION_CHARS)
                .unwrap_or_else(|| "（无描述）".to_string());
            format!("- {}: {}", skill.slug, description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 按字符数截断，超出时补省略号；内容为空时返回 None
fn truncate_chars(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut chars = trimmed.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        Some(format!("{head}…"))
    } else {
        Some(head)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(slug: &str) -> PmSkill {
        PmSkill {
            slug: slug.to_string(),
            name: slug.to_string(),
            description: format!("Description of {slug}"),
            content: format!("Body of {slug}"),
            source: "builtin".to_string(),
            size: 100,
        }
    }

    #[test]
    fn parse_skill_selection_reads_plain_array() {
        let skills = vec![skill("craft"), skill("double-diamond")];

        assert_eq!(
            parse_skill_selection("[\"craft\"]", &skills),
            vec!["craft".to_string()]
        );
        assert_eq!(
            parse_skill_selection("思考中…\n[\"double-diamond\", \"craft\"]", &skills),
            vec!["double-diamond".to_string(), "craft".to_string()]
        );
    }

    #[test]
    fn parse_skill_selection_dedupes_caps_and_drops_unknown() {
        let skills = vec![skill("craft"), skill("a"), skill("b"), skill("c"), skill("d")];

        assert_eq!(
            parse_skill_selection("[\"craft\",\"craft\",\"ghost\",\"a\"]", &skills),
            vec!["craft".to_string(), "a".to_string()]
        );
        assert_eq!(
            parse_skill_selection("[\"a\",\"b\",\"c\",\"d\"]", &skills).len(),
            MAX_SKILLS_PER_TURN
        );
    }

    #[test]
    fn parse_skill_selection_returns_empty_on_garbage() {
        let skills = vec![skill("craft")];

        assert!(parse_skill_selection("无法路由", &skills).is_empty());
        assert!(parse_skill_selection("[broken json", &skills).is_empty());
        assert!(parse_skill_selection("[]", &skills).is_empty());
        assert!(parse_skill_selection("[\"ghost\"]", &skills).is_empty());
    }

    #[test]
    fn build_skill_index_lists_all_slugs_with_truncated_descriptions() {
        let skills = vec![
            PmSkill {
                slug: "craft".to_string(),
                description: "short".to_string(),
                ..skill("craft")
            },
            PmSkill {
                slug: "empty".to_string(),
                description: String::new(),
                ..skill("empty")
            },
        ];

        let index = build_skill_index(&skills);

        assert!(index.contains("- craft: short"));
        assert!(index.contains("- empty: （无描述）"));
    }

    #[test]
    fn build_skill_index_handles_empty_library() {
        assert!(build_skill_index(&[]).contains("没有可用"));
    }

    #[test]
    fn truncate_chars_limits_by_chars_not_bytes() {
        let long = "一二三四五".to_string();

        assert_eq!(truncate_chars(&long, 3).as_deref(), Some("一二三…"));
        assert_eq!(truncate_chars("abc", 5).as_deref(), Some("abc"));
        assert_eq!(truncate_chars("   ", 5), None);
    }
}

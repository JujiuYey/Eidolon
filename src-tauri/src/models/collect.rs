//! 收藏夹 DTO。
//!
//! 低摩擦收集盒（对应 `collect_items` 表）：链接与随手记事统一存一张表，
//! `kind` 区分类型。链接保存时由后端异步抓取标题/摘要/favicon 回填。

use serde::{Deserialize, Serialize};

pub const COLLECT_KIND_LINK: &str = "link";
pub const COLLECT_KIND_NOTE: &str = "note";

/// 收藏夹 AI 功能使用的默认模型 key
pub const COLLECT_ASSISTANT_MODEL_KEY: &str = "collect_assistant";

/// 收藏条目（对应 `collect_items` 表）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectItem {
    #[serde(default)]
    pub id: String,
    /// link | note
    #[serde(default = "default_kind")]
    pub kind: String,
    /// 链接地址；记事为 None
    #[serde(default)]
    pub url: Option<String>,
    /// 标题：链接为抓取/手填标题；记事为 None
    #[serde(default)]
    pub title: Option<String>,
    /// 链接摘要（抓取的 description）；记事为 None
    #[serde(default)]
    pub description: Option<String>,
    /// 用户备注（链接）/ 正文（记事）
    #[serde(default)]
    pub content: String,
    /// 链接图标地址；记事为 None
    #[serde(default)]
    pub favicon_url: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// 链接元数据抓取结果（`fetch_link_metadata` 命令返回）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkMetadata {
    /// og:title / `<title>`，取不到为 None
    #[serde(default)]
    pub title: Option<String>,
    /// og:description / meta description，取不到为 None
    #[serde(default)]
    pub description: Option<String>,
    /// 页面声明的 favicon 绝对地址（页面未声明时回退 origin/favicon.ico）
    #[serde(default)]
    pub favicon_url: Option<String>,
}

/// 归一化类型：非法值回落为 note
pub fn normalize_kind(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case(COLLECT_KIND_LINK) {
        COLLECT_KIND_LINK.to_string()
    } else {
        COLLECT_KIND_NOTE.to_string()
    }
}

fn default_kind() -> String {
    COLLECT_KIND_NOTE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_item_deserializes_from_partial_json() {
        let item: CollectItem = serde_json::from_str(r#"{"content": "记点事"}"#).expect("partial json");
        assert_eq!(item.kind, "note", "kind should default to note");
        assert_eq!(item.content, "记点事");
        assert_eq!(item.url, None);
        assert!(!item.archived);
    }

    #[test]
    fn normalize_kind_falls_back_to_note() {
        assert_eq!(normalize_kind("link"), "link");
        assert_eq!(normalize_kind(" LINK "), "link");
        assert_eq!(normalize_kind("note"), "note");
        assert_eq!(normalize_kind("image"), "note");
        assert_eq!(normalize_kind(""), "note");
    }
}

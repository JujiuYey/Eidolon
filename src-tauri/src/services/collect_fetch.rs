//! 链接元数据抓取。
//!
//! 收藏链接时抓取网页标题（og:title → `<title>`）、摘要（og:description →
//! meta description）与 favicon（link rel=icon → origin/favicon.ico）。
//! 只处理 text/html 响应且 body 上限 2 MiB（防止误贴大文件链接拖死应用）；
//! 编码按 Content-Type charset → HTML meta charset → UTF-8 逐级确定，
//! 兼容 GBK 等未声明响应头的国内老站点。

use std::time::Duration;

use encoding_rs::Encoding;
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::models::collect::LinkMetadata;

/// 单请求超时
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// HTML body 上限：标题/摘要/favicon 都在页面头部，超限截断即可
const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;
/// meta charset 嗅探范围（HTML 规范要求声明出现在前 1024 字节，取两倍余量）
const CHARSET_SNIFF_BYTES: usize = 4096;
/// title/description 落库上限，避免异常页面写出超长字段
const MAX_TITLE_CHARS: usize = 256;
const MAX_DESCRIPTION_CHARS: usize = 2048;
/// 正文文本注入摘要 prompt 前的截断上限（字符）
const MAX_ARTICLE_CHARS: usize = 12_000;
/// 有效正文的最低字符数，低于视为提取失败（多半是 JS 渲染页）
const MIN_ARTICLE_CHARS: usize = 100;
/// 浏览器 UA：部分站点拒绝非浏览器 UA
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// 抓取链接的标题、摘要与 favicon。
/// 4xx/5xx、无法连接、非 http(s) 都返回 Err，由前端保留占位卡片。
pub async fn fetch_link_metadata(raw_url: &str) -> Result<LinkMetadata, String> {
    let parsed = parse_http_url(raw_url)?;
    let client = build_client()?;

    let (response, content_type) = get_html_response(&client, parsed).await?;

    // 重定向后的最终地址才是 favicon/相对路径的解析基准
    let final_url = response.url().clone();

    // 非 HTML（如误贴的 PDF/图片/大文件）不下载 body，只回退站点默认 favicon
    if !is_html_content_type(&content_type) {
        return Ok(LinkMetadata {
            favicon_url: default_favicon(&final_url),
            ..Default::default()
        });
    }

    let html = read_html(response, &content_type).await?;
    Ok(parse_metadata(&html, &final_url))
}

/// 抓取网页并提取正文文本（供 AI 摘要使用，不落库）
pub(crate) async fn fetch_page_text(raw_url: &str) -> Result<String, String> {
    let parsed = parse_http_url(raw_url)?;
    let client = build_client()?;

    let (response, content_type) = get_html_response(&client, parsed).await?;
    if !is_html_content_type(&content_type) {
        return Err("链接不是网页，无法提取正文".to_string());
    }

    let html = read_html(response, &content_type).await?;
    extract_article_text(&html)
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // 短链/站点根地址都可能重定向，允许有限跟随
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
}

/// GET 并校验状态，返回响应与 Content-Type（跟随重定向后的最终响应）
async fn get_html_response(
    client: &Client,
    url: Url,
) -> Result<(reqwest::Response, String), String> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("抓取网页失败: {error}"))?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();

    Ok((response, content_type))
}

fn parse_http_url(raw_url: &str) -> Result<Url, String> {
    let trimmed = raw_url.trim();
    let parsed = Url::parse(trimmed).map_err(|error| format!("无效的链接地址: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("仅支持 http/https 链接".to_string());
    }
    Ok(parsed)
}

fn is_html_content_type(content_type: &str) -> bool {
    let lower = content_type.to_ascii_lowercase();
    lower.starts_with("text/html") || lower.starts_with("application/xhtml+xml")
}

/// 流式读取 body，超过上限即停止（不下载剩余部分）
async fn read_html(
    mut response: reqwest::Response,
    content_type: &str,
) -> Result<String, String> {
    let mut bytes: Vec<u8> = Vec::new();
    while bytes.len() < MAX_HTML_BYTES {
        match response
            .chunk()
            .await
            .map_err(|error| format!("读取网页内容失败: {error}"))?
        {
            Some(chunk) => bytes.extend_from_slice(&chunk),
            None => break,
        }
    }
    Ok(decode_html(&bytes, content_type))
}

/// 编码判定：响应头 charset → HTML meta charset → UTF-8
fn decode_html(bytes: &[u8], content_type: &str) -> String {
    let encoding = header_charset(content_type)
        .or_else(|| meta_charset(bytes))
        .unwrap_or(encoding_rs::UTF_8);
    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

fn header_charset(content_type: &str) -> Option<&'static Encoding> {
    // to_ascii_lowercase 不改变字节长度，可安全地用小写副本定位、原文取值
    let index = content_type.to_ascii_lowercase().find("charset=")?;
    let rest = &content_type[index + "charset=".len()..];
    let label: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    Encoding::for_label(label.as_bytes())
}

fn meta_charset(bytes: &[u8]) -> Option<&'static Encoding> {
    let prefix = &bytes[..bytes.len().min(CHARSET_SNIFF_BYTES)];
    // charset 标签必为 ASCII，lossy 解码不影响匹配
    let lossy = String::from_utf8_lossy(prefix);
    let pattern = regex::Regex::new(r#"(?i)charset\s*=\s*["']?\s*([a-z0-9_\-]+)"#).ok()?;
    let captures = pattern.captures(&lossy)?;
    Encoding::for_label(captures.get(1)?.as_str().as_bytes())
}

/// 从已解码的 HTML 提取标题/摘要/favicon（纯函数，可单测）
fn parse_metadata(html: &str, base_url: &Url) -> LinkMetadata {
    let document = Html::parse_document(html);

    let title = meta_content(&document, r#"meta[property="og:title"]"#)
        .or_else(|| selector_text(&document, "title"))
        .map(|value| truncate_chars(value.trim(), MAX_TITLE_CHARS))
        .filter(|value| !value.is_empty());

    let description = meta_content(&document, r#"meta[property="og:description"]"#)
        .or_else(|| meta_content(&document, r#"meta[name="description"]"#))
        .map(|value| truncate_chars(value.trim(), MAX_DESCRIPTION_CHARS))
        .filter(|value| !value.is_empty());

    let favicon_url = favicon_from(&document, base_url).or_else(|| default_favicon(base_url));

    LinkMetadata {
        title,
        description,
        favicon_url,
    }
}

fn meta_content(document: &Html, selector: &str) -> Option<String> {
    let parsed = Selector::parse(selector).ok()?;
    document
        .select(&parsed)
        .find_map(|element| element.value().attr("content"))
        .map(str::to_string)
}

fn selector_text(document: &Html, selector: &str) -> Option<String> {
    let parsed = Selector::parse(selector).ok()?;
    document
        .select(&parsed)
        .next()
        .map(|element| element.text().collect::<String>())
}

/// 依次尝试 rel~=icon 与 rel~=apple-touch-icon，相对路径按最终 URL 解析
fn favicon_from(document: &Html, base_url: &Url) -> Option<String> {
    for selector in [r#"link[rel~="icon"]"#, r#"link[rel~="apple-touch-icon"]"#] {
        let parsed = match Selector::parse(selector) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        for element in document.select(&parsed) {
            let Some(href) = element.value().attr("href") else {
                continue;
            };
            let href = href.trim();
            if href.is_empty() || href.starts_with("data:") {
                continue;
            }
            if let Ok(resolved) = base_url.join(href) {
                return Some(resolved.to_string());
            }
        }
    }
    None
}

fn default_favicon(base_url: &Url) -> Option<String> {
    if base_url.scheme() != "http" && base_url.scheme() != "https" {
        return None;
    }
    Some(format!(
        "{}/favicon.ico",
        base_url.origin().ascii_serialization()
    ))
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max).collect();
    format!("{truncated}…")
}

/// 从 HTML 提取正文段落（纯函数，可单测）：p/h1-h3 按文档顺序拼接，
/// 有效字符数低于阈值视为失败（多半是 JS 渲染页拿不到文本）
fn extract_article_text(html: &str) -> Result<String, String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("p, h1, h2, h3")
        .map_err(|error| format!("解析选择器失败: {error}"))?;

    let mut lines: Vec<String> = Vec::new();
    for element in document.select(&selector) {
        let text = collapse_whitespace(&element.text().collect::<String>());
        if !text.is_empty() {
            lines.push(text);
        }
    }

    let joined = lines.join("\n");
    if joined.chars().count() < MIN_ARTICLE_CHARS {
        return Err("未能从页面提取到有效正文（页面可能是 JS 渲染或内容过短）".to_string());
    }

    Ok(truncate_chars(&joined, MAX_ARTICLE_CHARS))
}

/// 把元素内的连续空白（含换行）压成单个空格，段落间仍用换行分隔
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_url_accepts_only_http_schemes() {
        assert!(parse_http_url("https://example.com/a").is_ok());
        assert!(parse_http_url("  http://example.com  ").is_ok());
        assert!(parse_http_url("ftp://example.com").is_err());
        assert!(parse_http_url("file:///etc/hosts").is_err());
        assert!(parse_http_url("not a url").is_err());
    }

    #[test]
    fn is_html_content_type_matches_html_only() {
        assert!(is_html_content_type("text/html; charset=utf-8"));
        assert!(is_html_content_type("application/xhtml+xml"));
        assert!(!is_html_content_type("application/pdf"));
        assert!(!is_html_content_type("image/png"));
        assert!(!is_html_content_type(""));
    }

    #[test]
    fn decode_html_prefers_header_charset_over_meta() {
        let (gbk_bytes, _, _) = encoding_rs::GBK.encode("<html><body>抓取标题测试</body></html>");
        let header = "text/html; charset=GBK";

        assert_eq!(decode_html(&gbk_bytes, header), "<html><body>抓取标题测试</body></html>");
    }

    #[test]
    fn decode_html_sniffs_meta_charset_when_header_lacks_it() {
        let (gbk_bytes, _, _) =
            encoding_rs::GBK.encode(r#"<html><head><meta charset="gbk"></head><body>中文内容</body></html>"#);

        assert_eq!(
            decode_html(&gbk_bytes, "text/html"),
            r#"<html><head><meta charset="gbk"></head><body>中文内容</body></html>"#
        );
    }

    #[test]
    fn decode_html_falls_back_to_utf8() {
        let html = r#"<html><head><meta charset="utf-8"></head><body>默认编码</body></html>"#;
        assert_eq!(decode_html(html.as_bytes(), "text/html"), html);
    }

    #[test]
    fn parse_metadata_extracts_og_tags_and_resolves_relative_favicon() {
        let base = Url::parse("https://example.com/blog/post").expect("base url");
        let html = r#"<html><head>
            <meta property="og:title" content="一篇讲 Rust 的文章">
            <meta property="og:description" content="从零开始讲正文提取">
            <link rel="icon" href="/static/favicon.ico">
        </head><body></body></html>"#;

        let metadata = parse_metadata(html, &base);

        assert_eq!(metadata.title.as_deref(), Some("一篇讲 Rust 的文章"));
        assert_eq!(metadata.description.as_deref(), Some("从零开始讲正文提取"));
        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/static/favicon.ico")
        );
    }

    #[test]
    fn parse_metadata_falls_back_to_title_element_and_meta_description() {
        let base = Url::parse("https://example.com/").expect("base url");
        let html = r#"<html><head>
            <title>页面标题</title>
            <meta name="description" content="页面描述">
        </head><body></body></html>"#;

        let metadata = parse_metadata(html, &base);

        assert_eq!(metadata.title.as_deref(), Some("页面标题"));
        assert_eq!(metadata.description.as_deref(), Some("页面描述"));
    }

    #[test]
    fn parse_metadata_falls_back_to_origin_favicon_and_skips_data_urls() {
        let base = Url::parse("https://example.com/a/b").expect("base url");
        let html = r#"<html><head><link rel="icon" href="data:image/png;base64,AAAA"></head></html>"#;

        let metadata = parse_metadata(html, &base);

        assert_eq!(metadata.title, None);
        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.ico")
        );
    }

    #[test]
    fn parse_metadata_uses_final_url_host_for_favicon_fallback() {
        let base = Url::parse("https://cdn.example.net/").expect("base url");
        let metadata = parse_metadata("<html></html>", &base);
        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://cdn.example.net/favicon.ico")
        );
    }

    #[test]
    fn truncate_chars_caps_length_by_chars_not_bytes() {
        assert_eq!(truncate_chars("短标题", 10), "短标题");
        assert_eq!(truncate_chars("a".repeat(300).as_str(), 256).chars().count(), 257);
        let long = "中".repeat(300);
        let truncated = truncate_chars(&long, 256);
        assert_eq!(truncated.chars().count(), 257, "256 chars plus ellipsis");
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn extract_article_text_joins_paragraphs_in_document_order() {
        let html = r#"<html><body>
            <nav><p>站点导航</p></nav>
            <h1>正文标题</h1>
            <p>第一段：这是一段足够长的正文内容，用来撑过最低字符数要求，避免被当成提取失败的页面。</p>
            <blockquote><p>引用块里的段落也算正文段落。</p></blockquote>
            <p>第二段：收尾内容，补充说明与细节描述，让整体篇幅稳定超过一百个字符的门槛线。</p>
        </body></html>"#;

        let article = extract_article_text(html).expect("article text");

        assert!(article.contains("正文标题"), "heading should be kept, got {article}");
        assert!(article.contains("第一段"), "paragraphs should be kept");
        assert!(article.contains("引用块里的段落"), "p inside blockquote still matches p selector");
        assert!(article.contains("第二段"));
        // p 文本内部的换行/缩进被压成空格
        assert!(!article.contains("  "), "whitespace should collapse, got {article}");
    }

    #[test]
    fn extract_article_text_rejects_pages_without_readable_text() {
        // 纯脚本/空壳页面：没有 p/h1-h3 文本
        assert!(extract_article_text("<html><head><title>x</title></head><body></body></html>").is_err());
        // 文本过短
        assert!(extract_article_text("<html><body><p>太短了</p></body></html>").is_err());
    }

    #[test]
    fn extract_article_text_caps_length() {
        let paragraph = format!("<p>{}</p>", "字".repeat(5000));
        // 5000 字 x4 段 = 20000 字，超过 12000 上限
        let html = format!("<html><body>{paragraph}{paragraph}{paragraph}{paragraph}</body></html>");

        let article = extract_article_text(&html).expect("article text");
        assert_eq!(article.chars().count(), 12_001, "12000 chars plus ellipsis");
        assert!(article.ends_with('…'));
    }
}

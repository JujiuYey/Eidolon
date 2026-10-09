use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 一个已加载的方法论框架
#[derive(Debug, Clone, PartialEq)]
pub struct PmSkill {
    /// 文件夹名，同时是路由与覆盖合并的键
    pub slug: String,
    /// frontmatter 中的 name，解析失败时回退为 slug
    pub name: String,
    /// frontmatter 中的 description（支持多行折叠与引号包裹）
    pub description: String,
    /// 去掉 frontmatter 后的正文，注入提示词时使用
    pub content: String,
    /// "builtin"（随应用打包）或 "override"（来自覆盖目录）
    pub source: String,
    /// SKILL.md 原始字节数
    pub size: u64,
}

/// 加载技能库：先扫描内置目录，再扫描覆盖目录；同名（按文件夹名）时覆盖版本胜出。
/// 任何一个目录缺失或不可读都只是跳过，不报错——技能库是增强能力而非硬依赖。
pub fn load_skill_library(builtin_dir: &Path, override_dir: Option<&str>) -> Vec<PmSkill> {
    let mut skills: HashMap<String, PmSkill> = HashMap::new();

    scan_skill_dir(builtin_dir, "builtin", &mut skills);

    if let Some(dir) = override_dir
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
    {
        let path = std::path::PathBuf::from(dir);
        if path.is_dir() {
            scan_skill_dir(&path, "override", &mut skills);
        }
    }

    let mut list: Vec<PmSkill> = skills.into_values().collect();
    list.sort_by(|left, right| left.slug.cmp(&right.slug));
    list
}

fn scan_skill_dir(root: &Path, source: &str, into: &mut HashMap<String, PmSkill>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let skill_path = entry.path().join("SKILL.md");
        if !skill_path.is_file() {
            continue;
        }

        let Ok(raw) = fs::read_to_string(&skill_path) else {
            continue;
        };

        let slug = entry.file_name().to_string_lossy().to_string();
        let (name, description, content) = split_frontmatter(&raw, &slug);

        into.insert(
            slug.clone(),
            PmSkill {
                slug,
                name,
                description,
                content,
                source: source.to_string(),
                size: raw.len() as u64,
            },
        );
    }
}

/// 解析 `---` 围起来的 frontmatter 块，返回 (name, description, 正文)。
/// 手写解析以避免引入 YAML 依赖；需覆盖 awesome-ux-skills 实际出现的三种形态：
/// 单行值、多行折叠（缩进续行）、带引号值，以及夹带的额外键（如 license）。
pub fn split_frontmatter(raw: &str, slug: &str) -> (String, String, String) {
    let lines: Vec<&str> = raw.lines().collect();

    let starts_fence = lines.first().copied().map(str::trim_end) == Some("---");
    if !starts_fence {
        return (slug.to_string(), String::new(), raw.trim().to_string());
    }

    // 闭合围栏：从第二行起第一个内容恰为 "---" 的行
    let close = lines
        .iter()
        .skip(1)
        .position(|line| line.trim_end() == "---");

    let Some(close) = close else {
        return (slug.to_string(), String::new(), raw.trim().to_string());
    };

    let block = &lines[1..=close];
    let body = lines[close + 2..].join("\n").trim().to_string();

    let (name, description) = parse_block(block, slug);
    (name, description, body)
}

fn parse_block(block: &[&str], slug: &str) -> (String, String) {
    let mut name = String::new();
    let mut description = String::new();
    let mut in_description = false;

    for line in block {
        if let Some(rest) = key_value(line, "name") {
            name = rest.trim().to_string();
            in_description = false;
            continue;
        }

        if let Some(rest) = key_value(line, "description") {
            description = rest.trim().to_string();
            in_description = true;
            continue;
        }

        if in_description {
            if line.starts_with(' ') || line.starts_with('\t') {
                // 多行折叠标量的续行
                let part = line.trim();
                if !part.is_empty() {
                    if !description.is_empty() {
                        description.push(' ');
                    }
                    description.push_str(part);
                }
            } else {
                // 顶格新键（如 license）或空行，描述结束
                in_description = false;
            }
        }
    }

    let name = if name.is_empty() {
        slug.to_string()
    } else {
        strip_quotes(&name)
    };

    (name, strip_quotes(&description))
}

/// 匹配顶格的 `key: value` 行，返回值部分
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    let rest = line.strip_prefix(&prefix)?;
    if rest.is_empty() || rest.starts_with(' ') {
        Some(rest)
    } else {
        None
    }
}

fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let first = bytes[0];
        let last = bytes[trimmed.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_parses_single_line_values() {
        let raw = "---\nname: craft\ndescription: Apply 12 concrete rules.\n---\n\n# Craft\n\nBody text.";

        let (name, description, content) = split_frontmatter(raw, "fallback");

        assert_eq!(name, "craft");
        assert_eq!(description, "Apply 12 concrete rules.");
        assert_eq!(content, "# Craft\n\nBody text.");
    }

    #[test]
    fn split_frontmatter_folds_multi_line_description() {
        let raw = "---\nname: empathy-mapping\ndescription: Create, facilitate, and critique\n  empathy maps for UX research.\n  Second folded line.\n---\n\nBody.";

        let (name, description, content) = split_frontmatter(raw, "fallback");

        assert_eq!(name, "empathy-mapping");
        assert_eq!(
            description,
            "Create, facilitate, and critique empathy maps for UX research. Second folded line."
        );
        assert_eq!(content, "Body.");
    }

    #[test]
    fn split_frontmatter_strips_wrapped_quotes_and_tolerates_extra_keys() {
        let raw = "---\nname: ui-ux-pro-max\ndescription: \"UI/UX design intelligence.\"\nlicense: Complete terms in LICENSE.txt\n---\n\nBody.";

        let (name, description, content) = split_frontmatter(raw, "fallback");

        assert_eq!(name, "ui-ux-pro-max");
        assert_eq!(description, "UI/UX design intelligence.");
        assert_eq!(content, "Body.");
    }

    #[test]
    fn split_frontmatter_description_before_name_is_kept() {
        let raw = "---\ndescription: Desc first\nname: later-name\n---\n\nBody.";

        let (name, description, _) = split_frontmatter(raw, "fallback");

        assert_eq!(name, "later-name");
        assert_eq!(description, "Desc first");
    }

    #[test]
    fn split_frontmatter_falls_back_to_slug_without_block() {
        let no_fence = "# Just a heading\n\nBody.";
        let unclosed = "---\nname: broken\ndescription: no closing fence";

        assert_eq!(split_frontmatter(no_fence, "slug-a").0, "slug-a");
        assert_eq!(split_frontmatter(unclosed, "slug-b").0, "slug-b");
        assert_eq!(split_frontmatter(no_fence, "slug-a").2, no_fence.trim());
    }

    #[test]
    fn split_frontmatter_defaults_name_to_slug_when_missing() {
        let raw = "---\ndescription: Only description here.\n---\n\nBody.";

        let (name, description, _) = split_frontmatter(raw, "slug-c");

        assert_eq!(name, "slug-c");
        assert_eq!(description, "Only description here.");
    }

    #[test]
    fn load_skill_library_merges_override_by_slug() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let builtin = temp.path().join("builtin");
        let r#override = temp.path().join("override");

        fs::create_dir_all(builtin.join("craft")).expect("dir should be created");
        fs::create_dir_all(builtin.join("persuasive-ux")).expect("dir should be created");
        fs::create_dir_all(r#override.join("craft")).expect("dir should be created");
        fs::create_dir_all(r#override.join("my-custom")).expect("dir should be created");

        fs::write(
            builtin.join("craft").join("SKILL.md"),
            "---\nname: craft\ndescription: builtin version\n---\n\nbuiltin body",
        )
        .expect("write should succeed");
        fs::write(
            builtin.join("persuasive-ux").join("SKILL.md"),
            "---\nname: persuasive-ux\ndescription: builtin only\n---\n\nbuiltin body",
        )
        .expect("write should succeed");
        fs::write(
            r#override.join("craft").join("SKILL.md"),
            "---\nname: craft\ndescription: overridden version\n---\n\noverride body",
        )
        .expect("write should succeed");
        fs::write(
            r#override.join("my-custom").join("SKILL.md"),
            "---\nname: my-custom\ndescription: extra skill\n---\n\ncustom body",
        )
        .expect("write should succeed");

        let skills = load_skill_library(&builtin, Some(r#override.to_str().unwrap()));

        assert_eq!(skills.len(), 3);

        let craft = skills.iter().find(|s| s.slug == "craft").expect("craft exists");
        assert_eq!(craft.source, "override");
        assert_eq!(craft.description, "overridden version");
        assert_eq!(craft.content, "override body");

        let persuasive = skills
            .iter()
            .find(|s| s.slug == "persuasive-ux")
            .expect("persuasive-ux exists");
        assert_eq!(persuasive.source, "builtin");

        let custom = skills
            .iter()
            .find(|s| s.slug == "my-custom")
            .expect("custom exists");
        assert_eq!(custom.source, "override");
    }

    #[test]
    fn load_skill_library_tolerates_missing_dirs() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let missing = temp.path().join("does-not-exist");

        let skills = load_skill_library(&missing, Some("/definitely/not/here"));

        assert!(skills.is_empty());
    }
}

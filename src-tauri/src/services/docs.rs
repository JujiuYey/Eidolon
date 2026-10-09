use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::db::local_store::LocalJsonStore;
use crate::models::docs::{DocsEntry, DocsSearchResult, DocsSettings};
use crate::services::work_directory::assert_path_within_work_directory;

const DOCS_SETTINGS_FILENAME: &str = "docs_settings";
/// 单文件读取上限，防止误把超大文件拉进编辑器
const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
/// 全文搜索命中行上限
const MAX_SEARCH_MATCHES: usize = 200;
/// 跳过的目录名（含 Obsidian 的配置目录，避免污染文档树）
const SKIP_DIRS: [&str; 4] = [".git", "node_modules", ".obsidian", ".trash"];

pub fn get_settings(store: &LocalJsonStore) -> DocsSettings {
    store.read(DOCS_SETTINGS_FILENAME).unwrap_or_default()
}

/// 校验并保存设置。目录必须存在且确实是目录，避免配置一个拼错的路径
pub fn save_settings(store: &LocalJsonStore, settings: &DocsSettings) -> Result<DocsSettings, String> {
    let normalized = match settings.root_dir.as_deref().map(str::trim) {
        None | Some("") => DocsSettings { root_dir: None },
        Some(dir) => {
            let path = PathBuf::from(dir);
            if !path.is_absolute() {
                return Err("文档库目录必须是绝对路径".to_string());
            }
            if !path.is_dir() {
                return Err(format!("目录不存在或不是文件夹: {dir}"));
            }
            DocsSettings { root_dir: Some(dir.to_string()) }
        }
    };

    store.write(DOCS_SETTINGS_FILENAME, &normalized)?;
    Ok(normalized)
}

fn resolve_root(store: &LocalJsonStore) -> Result<PathBuf, String> {
    let settings = get_settings(store);
    let root = settings
        .root_dir
        .ok_or_else(|| "请先在「文档」页选择文档库目录".to_string())?;
    let path = PathBuf::from(&root);
    if !path.is_dir() {
        return Err(format!("文档库目录不可用: {root}"));
    }
    Ok(path)
}

/// 所有文件操作的统一出口：根目录 + 相对路径 → 沙箱内的绝对路径
fn sandboxed_path(store: &LocalJsonStore, relative: &str) -> Result<PathBuf, String> {
    let root = resolve_root(store)?;
    assert_path_within_work_directory(&root.to_string_lossy(), Path::new(relative))
}

fn to_relative_path(root: &Path, absolute: &Path) -> String {
    let root_string = root.to_string_lossy();
    let prefix = format!("{}/", root_string.trim_end_matches('/'));
    let full = absolute.to_string_lossy();
    full.strip_prefix(prefix.as_str())
        .unwrap_or(full.as_ref())
        .to_string()
}

fn build_entry(root: &Path, absolute: &Path, is_dir: bool) -> Option<DocsEntry> {
    let metadata = fs::metadata(absolute).ok()?;
    let updated_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Some(DocsEntry {
        name: absolute.file_name()?.to_string_lossy().to_string(),
        path: to_relative_path(root, absolute),
        is_dir,
        size: if is_dir { 0 } else { metadata.len() },
        updated_at,
    })
}

fn is_skipped(name: &str, is_dir: bool) -> bool {
    let lower = name.to_lowercase();
    if is_dir {
        SKIP_DIRS.contains(&lower.as_str())
    } else {
        !lower.ends_with(".md") || name.starts_with('.') || name == ".DS_Store"
    }
}

/// 列出全部条目（仅 .md 文件 + 文件夹），目录在前、同层按名称排序。
/// 返回扁平列表，树形结构由前端构建。
pub fn list_entries(store: &LocalJsonStore) -> Result<Vec<DocsEntry>, String> {
    let root = resolve_root(store)?;

    let mut entries = Vec::new();
    for item in WalkDir::new(&root)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !is_skipped(&entry.file_name().to_string_lossy(), entry.file_type().is_dir())
        })
        .flatten()
    {
        if item.file_type().is_dir() {
            if let Some(entry) = build_entry(&root, item.path(), true) {
                entries.push(entry);
            }
        } else if let Some(entry) = build_entry(&root, item.path(), false) {
            entries.push(entry);
        }
    }

    // walkdir 的 sort_by_file_name 只保证局部序，这里做全局稳定排序：目录在前
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(entries)
}

pub fn read_file(store: &LocalJsonStore, relative: &str) -> Result<String, String> {
    let path = sandboxed_path(store, relative)?;
    if !path.is_file() {
        return Err(format!("文件不存在: {relative}"));
    }

    let metadata = fs::metadata(&path).map_err(|error| format!("读取文件信息失败: {error}"))?;
    if metadata.len() > MAX_READ_BYTES {
        return Err("文件超过 2MB，暂不支持在应用内编辑".to_string());
    }

    fs::read_to_string(&path).map_err(|error| format!("读取文件失败: {error}"))
}

pub fn save_file(store: &LocalJsonStore, relative: &str, content: &str) -> Result<DocsEntry, String> {
    let path = sandboxed_path(store, relative)?;
    if path.is_dir() {
        return Err("目标是文件夹，不能保存为文件".to_string());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建父目录失败: {error}"))?;
    }

    fs::write(&path, content).map_err(|error| format!("写入文件失败: {error}"))?;
    let root = resolve_root(store)?;
    build_entry(&root, &path, false).ok_or_else(|| "保存后读取文件信息失败".to_string())
}

/// 清理用户输入的名称：去首尾空白、拒绝路径分隔符和点开头的隐藏名；
/// 文件自动补 .md 后缀
fn sanitize_name(name: &str, is_dir: bool) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if trimmed.starts_with('.') {
        return Err("名称不能以点开头".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." || trimmed == ".." {
        return Err("名称不能包含路径分隔符".to_string());
    }

    if is_dir || trimmed.to_lowercase().ends_with(".md") {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("{trimmed}.md"))
    }
}

pub fn create_file(store: &LocalJsonStore, dir_relative: &str, name: &str) -> Result<DocsEntry, String> {
    let file_name = sanitize_name(name, false)?;
    let parent = sandboxed_path(store, dir_relative)?;
    if !parent.is_dir() {
        return Err(format!("目标文件夹不存在: {dir_relative}"));
    }

    let path = parent.join(&file_name);
    if path.exists() {
        return Err(format!("已存在同名文件: {file_name}"));
    }

    fs::write(&path, "").map_err(|error| format!("创建文件失败: {error}"))?;
    let root = resolve_root(store)?;
    build_entry(&root, &path, false).ok_or_else(|| "创建后读取文件信息失败".to_string())
}

pub fn create_directory(store: &LocalJsonStore, parent_relative: &str, name: &str) -> Result<DocsEntry, String> {
    let dir_name = sanitize_name(name, true)?;
    let parent = sandboxed_path(store, parent_relative)?;
    if !parent.is_dir() {
        return Err(format!("目标文件夹不存在: {parent_relative}"));
    }

    let path = parent.join(&dir_name);
    if path.exists() {
        return Err(format!("已存在同名文件夹: {dir_name}"));
    }

    fs::create_dir_all(&path).map_err(|error| format!("创建文件夹失败: {error}"))?;
    let root = resolve_root(store)?;
    build_entry(&root, &path, true).ok_or_else(|| "创建后读取文件夹信息失败".to_string())
}

/// 重命名文件或文件夹。文件原本带 .md 时自动为新名称补上
pub fn rename_entry(store: &LocalJsonStore, relative: &str, new_name: &str) -> Result<DocsEntry, String> {
    let path = sandboxed_path(store, relative)?;
    if !path.exists() {
        return Err(format!("条目不存在: {relative}"));
    }
    let is_dir = path.is_dir();

    let raw_name = new_name.trim();
    let final_name = if !is_dir
        && relative.to_lowercase().ends_with(".md")
        && !raw_name.to_lowercase().ends_with(".md")
    {
        format!("{raw_name}.md")
    } else {
        raw_name.to_string()
    };
    let final_name = sanitize_name(&final_name, is_dir)?;

    let target = path
        .parent()
        .ok_or_else(|| "无法定位父目录".to_string())?
        .join(&final_name);
    if target == path {
        let root = resolve_root(store)?;
        return build_entry(&root, &path, is_dir).ok_or_else(|| "读取条目信息失败".to_string());
    }
    if target.exists() {
        return Err(format!("已存在同名条目: {final_name}"));
    }

    fs::rename(&path, &target).map_err(|error| format!("重命名失败: {error}"))?;
    let root = resolve_root(store)?;
    build_entry(&root, &target, is_dir).ok_or_else(|| "重命名后读取条目信息失败".to_string())
}

/// 删除文件；文件夹必须为空才允许删，避免一键误删整棵子树
pub fn delete_entry(store: &LocalJsonStore, relative: &str) -> Result<String, String> {
    let path = sandboxed_path(store, relative)?;
    if !path.exists() {
        return Err(format!("条目不存在: {relative}"));
    }

    if path.is_dir() {
        let is_empty = fs::read_dir(&path)
            .map(|mut entries| entries.next().is_none())
            .map_err(|error| format!("读取文件夹失败: {error}"))?;
        if !is_empty {
            return Err("文件夹不为空，请先清空或逐个删除里面的文档".to_string());
        }
        fs::remove_dir(&path).map_err(|error| format!("删除文件夹失败: {error}"))?;
    } else {
        fs::remove_file(&path).map_err(|error| format!("删除文件失败: {error}"))?;
    }

    Ok(relative.to_string())
}

/// 全文搜索：遍历 .md 文件逐行匹配（大小写不敏感），带命中上限
pub fn search(store: &LocalJsonStore, keyword: &str) -> Result<Vec<DocsSearchResult>, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    let keyword_lower = keyword.to_lowercase();
    let root = resolve_root(store)?;

    let mut results = Vec::new();
    for item in WalkDir::new(&root)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !is_skipped(&entry.file_name().to_string_lossy(), entry.file_type().is_dir())
        })
        .flatten()
    {
        if !item.file_type().is_file() {
            continue;
        }
        if results.len() >= MAX_SEARCH_MATCHES {
            break;
        }

        let Ok(content) = fs::read_to_string(item.path()) else {
            continue;
        };
        let name = item.file_name().to_string_lossy().to_string();
        let path = to_relative_path(&root, item.path());

        for (index, line) in content.lines().enumerate() {
            if results.len() >= MAX_SEARCH_MATCHES {
                break;
            }
            if line.to_lowercase().contains(&keyword_lower) {
                results.push(DocsSearchResult {
                    path: path.clone(),
                    name: name.clone(),
                    line_number: index as u32 + 1,
                    line_text: line.trim().chars().take(160).collect(),
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, LocalJsonStore) {
        let docs_dir = tempdir().expect("docs temp dir");
        let store_dir = tempdir().expect("store temp dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");
        let saved = save_settings(
            &store,
            &DocsSettings { root_dir: Some(docs_dir.path().to_string_lossy().to_string()) },
        )
        .expect("settings save");
        assert!(saved.root_dir.is_some());
        (docs_dir, store_dir, store)
    }

    fn write_docs_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn settings_roundtrip_and_validation() {
        let (_docs, store_dir, store) = setup();
        assert_eq!(
            get_settings(&store).root_dir,
            Some(_docs.path().to_string_lossy().to_string())
        );

        let bad = save_settings(&store, &DocsSettings { root_dir: Some("relative/path".into()) });
        assert!(bad.is_err());

        let empty = save_settings(&store, &DocsSettings { root_dir: Some("   ".into()) }).expect("blank clears");
        assert_eq!(empty.root_dir, None);

        let fresh = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("fresh store");
        assert_eq!(get_settings(&fresh).root_dir, None);
    }

    #[test]
    fn list_entries_orders_dirs_first_and_filters_junk() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();

        write_docs_file(root, "周报/2026-10.md", "# 周报");
        write_docs_file(root, "readme.md", "hello");
        write_docs_file(root, ".hidden.md", "x");
        write_docs_file(root, "notes.txt", "not markdown");
        fs::create_dir_all(root.join(".git")).expect("git dir");
        fs::write(root.join(".git/config"), "x").expect("git file");

        let entries = list_entries(&store).expect("entries");

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"周报"));
        assert!(names.contains(&"readme.md"));
        assert!(names.contains(&"2026-10.md"));
        assert!(!names.iter().any(|&n| n == ".hidden.md" || n == "notes.txt" || n == ".git" || n == "config"));

        assert_eq!(entries[0].is_dir, true);
        assert_eq!(entries[0].name, "周报");
        let readme = entries.iter().find(|e| e.name == "readme.md").expect("readme");
        assert_eq!(readme.path, "readme.md");
        assert_eq!(readme.size, 5);
        assert!(readme.updated_at > 0);
    }

    #[test]
    fn read_file_enforces_size_limit_and_sandbox() {
        let (docs, _store_dir, store) = setup();
        write_docs_file(docs.path(), "big.md", &"x".repeat(3 * 1024 * 1024));

        assert!(read_file(&store, "big.md").is_err());
        assert!(read_file(&store, "../outside.md").is_err());
        assert!(read_file(&store, "missing.md").is_err());
    }

    #[test]
    fn create_file_appends_md_and_rejects_collisions() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();
        write_docs_file(root, "note.md", "old");

        let created = create_file(&store, "", "新文档").expect("create");
        assert_eq!(created.name, "新文档.md");
        assert_eq!(created.path, "新文档.md");
        assert!(root.join("新文档.md").exists());

        assert!(create_file(&store, "", "note").is_err()); // 补 .md 后与 note.md 冲突
        assert!(create_file(&store, "", "../逃逸").is_err()); // 名称带分隔符被拒
        assert!(create_file(&store, "", ".隐藏").is_err());
    }

    #[test]
    fn save_and_rename_and_delete_roundtrip() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();
        let entry = create_file(&store, "", "草稿").expect("create");

        let saved = save_file(&store, &entry.path, "# 标题\n内容").expect("save");
        assert_eq!(saved.size, "# 标题\n内容".len() as u64);
        assert_eq!(read_file(&store, "草稿.md").expect("read"), "# 标题\n内容");

        let renamed = rename_entry(&store, "草稿.md", "正式文档").expect("rename");
        assert_eq!(renamed.path, "正式文档.md");
        assert!(!root.join("草稿.md").exists());
        assert_eq!(read_file(&store, "正式文档.md").expect("read again"), "# 标题\n内容");

        assert_eq!(delete_entry(&store, "正式文档.md").expect("delete"), "正式文档.md");
        assert!(!root.join("正式文档.md").exists());
    }

    #[test]
    fn rename_file_keeps_md_suffix_and_blocks_collision() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();
        write_docs_file(root, "a.md", "a");
        write_docs_file(root, "b.md", "b");

        let renamed = rename_entry(&store, "a.md", "新名字").expect("rename");
        assert_eq!(renamed.name, "新名字.md");

        assert!(rename_entry(&store, "新名字.md", "b").is_err()); // 会撞 b.md
        assert!(rename_entry(&store, "missing.md", "x").is_err());
    }

    #[test]
    fn delete_directory_requires_empty() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();
        write_docs_file(root, "folder/inner.md", "x");

        assert!(delete_entry(&store, "folder").is_err());
        fs::remove_file(root.join("folder/inner.md")).expect("clear");
        assert!(delete_entry(&store, "folder").is_ok());
    }

    #[test]
    fn search_finds_lines_case_insensitively() {
        let (docs, _store_dir, store) = setup();
        let root = docs.path();
        write_docs_file(root, "notes/api.md", "# API\n调用的接口是 sendEmail\n");
        write_docs_file(root, "notes/other.md", "无关内容");

        let results = search(&store, "sendemail").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "notes/api.md");
        assert_eq!(results[0].name, "api.md");
        assert_eq!(results[0].line_number, 2);
        assert!(results[0].line_text.contains("sendEmail"));

        assert!(search(&store, "  ").expect("blank").is_empty());
    }

    #[test]
    fn operations_fail_without_root() {
        let store_dir = tempdir().expect("store dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");

        assert!(list_entries(&store).is_err());
        assert!(read_file(&store, "a.md").is_err());
        assert!(save_file(&store, "a.md", "x").is_err());
    }
}

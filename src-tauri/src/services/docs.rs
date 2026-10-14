use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::db::local_store::LocalJsonStore;
use crate::models::docs::{DocsEntry, DocsRoot, DocsSearchResult, DocsSettings};
use crate::services::work_directory::assert_path_within_work_directory;

const DOCS_SETTINGS_FILENAME: &str = "docs_settings";
/// 单文件读取上限，防止误把超大文件拉进编辑器
const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
/// 全文搜索命中行上限
const MAX_SEARCH_MATCHES: usize = 200;
/// 跳过的目录名（含 Obsidian 的配置目录，避免污染文档树）
const SKIP_DIRS: [&str; 4] = [".git", "node_modules", ".obsidian", ".trash"];

pub fn get_settings(store: &LocalJsonStore) -> DocsSettings {
    let mut settings: DocsSettings = store.read(DOCS_SETTINGS_FILENAME).unwrap_or_default();
    if settings.roots.is_empty() {
        // 旧版单根配置折叠成第一个文档库；id 确定性生成，读取即迁移，无需落盘
        if let Some(dir) = settings.root_dir.as_deref().map(str::trim).filter(|dir| !dir.is_empty()) {
            settings.roots.push(DocsRoot { id: root_id_for(dir), path: dir.to_string() });
        }
    }
    settings
}

/// 校验并保存设置。每个库必须是存在的绝对路径，重复路径静默去重
pub fn save_settings(store: &LocalJsonStore, settings: &DocsSettings) -> Result<DocsSettings, String> {
    let mut roots: Vec<DocsRoot> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for root in &settings.roots {
        let dir = root.path.trim();
        if dir.is_empty() {
            return Err("文档库路径不能为空".to_string());
        }
        let path = PathBuf::from(dir);
        if !path.is_absolute() {
            return Err("文档库目录必须是绝对路径".to_string());
        }
        if !path.is_dir() {
            return Err(format!("目录不存在或不是文件夹: {dir}"));
        }

        let key = dir.trim_end_matches('/').to_string();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        roots.push(DocsRoot { id: root_id_for(dir), path: dir.to_string() });
    }

    let normalized = DocsSettings { root_dir: None, roots };
    store.write(DOCS_SETTINGS_FILENAME, &normalized)?;
    Ok(normalized)
}

/// fnv-1a：路径 → 确定性的库 id，同一目录每次生成的 id 一致，可安全用作路由参数
fn root_id_for(path: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn resolve_root(store: &LocalJsonStore, root_id: &str) -> Result<PathBuf, String> {
    let settings = get_settings(store);
    let root = settings
        .roots
        .iter()
        .find(|root| root.id == root_id)
        .ok_or_else(|| "文档库不存在或已被移除".to_string())?;
    let path = PathBuf::from(&root.path);
    if !path.is_dir() {
        return Err(format!("文档库目录不可用: {}", root.path));
    }
    Ok(path)
}

/// 相对路径 → 沙箱内的绝对路径
fn sandboxed_path_with_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
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

/// 列出指定库的全部条目（仅 .md 文件 + 文件夹），目录在前、同层按名称排序。
/// 返回相对库根的扁平列表，树形结构由前端构建。
pub fn list_entries(store: &LocalJsonStore, root_id: &str) -> Result<Vec<DocsEntry>, String> {
    let root = resolve_root(store, root_id)?;

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

pub fn read_file(store: &LocalJsonStore, root_id: &str, relative: &str) -> Result<String, String> {
    let root = resolve_root(store, root_id)?;
    let path = sandboxed_path_with_root(&root, relative)?;
    if !path.is_file() {
        return Err(format!("文件不存在: {relative}"));
    }

    let metadata = fs::metadata(&path).map_err(|error| format!("读取文件信息失败: {error}"))?;
    if metadata.len() > MAX_READ_BYTES {
        return Err("文件超过 2MB，暂不支持在应用内编辑".to_string());
    }

    fs::read_to_string(&path).map_err(|error| format!("读取文件失败: {error}"))
}

pub fn save_file(store: &LocalJsonStore, root_id: &str, relative: &str, content: &str) -> Result<DocsEntry, String> {
    let root = resolve_root(store, root_id)?;
    let path = sandboxed_path_with_root(&root, relative)?;
    if path.is_dir() {
        return Err("目标是文件夹，不能保存为文件".to_string());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建父目录失败: {error}"))?;
    }

    fs::write(&path, content).map_err(|error| format!("写入文件失败: {error}"))?;
    build_entry(&root, &path, false).ok_or_else(|| "保存后读取文件信息失败".to_string())
}

/// 在系统文件管理器中定位该条目（macOS 访达里选中它，Windows 资源管理器同理）。
/// 路径经沙箱解析，不能借此访问文档库之外的任意路径。
pub fn reveal_entry(store: &LocalJsonStore, root_id: &str, relative: &str) -> Result<(), String> {
    let root = resolve_root(store, root_id)?;
    let path = sandboxed_path_with_root(&root, relative)?;
    if !path.exists() {
        return Err(format!("路径不存在: {relative}"));
    }

    // 目录直接打开；文件要「定位并选中」，否则 open 会用默认应用把它打开
    if path.is_dir() {
        return open_in_file_manager(&path, false);
    }
    open_in_file_manager(&path, true)
}

fn open_in_file_manager(path: &Path, select: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("open");
        if select {
            command.arg("-R");
        }
        command
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开访达失败: {error}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new("explorer");
        if select {
            // 资源管理器只认 /select, 必须和路径拼成一个参数
            command.arg(format!("/select,{}", path.to_string_lossy()));
        } else {
            command.arg(path);
        }
        command
            .spawn()
            .map_err(|error| format!("打开资源管理器失败: {error}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Linux 的文件管理器没有统一的「选中」约定，退化为打开所在目录
        let target = if select {
            path.parent().unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        };
        std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(|error| format!("打开文件管理器失败: {error}"))?;
    }

    Ok(())
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

pub fn create_file(store: &LocalJsonStore, root_id: &str, dir_relative: &str, name: &str) -> Result<DocsEntry, String> {
    let file_name = sanitize_name(name, false)?;
    let root = resolve_root(store, root_id)?;
    let parent = sandboxed_path_with_root(&root, dir_relative)?;
    if !parent.is_dir() {
        return Err(format!("目标文件夹不存在: {dir_relative}"));
    }

    let path = parent.join(&file_name);
    if path.exists() {
        return Err(format!("已存在同名文件: {file_name}"));
    }

    fs::write(&path, "").map_err(|error| format!("创建文件失败: {error}"))?;
    build_entry(&root, &path, false).ok_or_else(|| "创建后读取文件信息失败".to_string())
}

pub fn create_directory(
    store: &LocalJsonStore,
    root_id: &str,
    parent_relative: &str,
    name: &str,
) -> Result<DocsEntry, String> {
    let dir_name = sanitize_name(name, true)?;
    let root = resolve_root(store, root_id)?;
    let parent = sandboxed_path_with_root(&root, parent_relative)?;
    if !parent.is_dir() {
        return Err(format!("目标文件夹不存在: {parent_relative}"));
    }

    let path = parent.join(&dir_name);
    if path.exists() {
        return Err(format!("已存在同名文件夹: {dir_name}"));
    }

    fs::create_dir_all(&path).map_err(|error| format!("创建文件夹失败: {error}"))?;
    build_entry(&root, &path, true).ok_or_else(|| "创建后读取文件夹信息失败".to_string())
}

/// 重命名文件或文件夹。文件原本带 .md 时自动为新名称补上
pub fn rename_entry(store: &LocalJsonStore, root_id: &str, relative: &str, new_name: &str) -> Result<DocsEntry, String> {
    let root = resolve_root(store, root_id)?;
    let path = sandboxed_path_with_root(&root, relative)?;
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
        return build_entry(&root, &path, is_dir).ok_or_else(|| "读取条目信息失败".to_string());
    }
    if target.exists() {
        return Err(format!("已存在同名条目: {final_name}"));
    }

    fs::rename(&path, &target).map_err(|error| format!("重命名失败: {error}"))?;
    build_entry(&root, &target, is_dir).ok_or_else(|| "重命名后读取条目信息失败".to_string())
}

/// 删除文件；文件夹必须为空才允许删，避免一键误删整棵子树
pub fn delete_entry(store: &LocalJsonStore, root_id: &str, relative: &str) -> Result<String, String> {
    let root = resolve_root(store, root_id)?;
    let path = sandboxed_path_with_root(&root, relative)?;
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

/// 全文搜索（仅限指定库）：遍历 .md 文件逐行匹配（大小写不敏感），带命中上限
pub fn search(store: &LocalJsonStore, root_id: &str, keyword: &str) -> Result<Vec<DocsSearchResult>, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    let keyword_lower = keyword.to_lowercase();
    let root = resolve_root(store, root_id)?;

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

    struct TestContext {
        docs: tempfile::TempDir,
        _store_dir: tempfile::TempDir,
        store: LocalJsonStore,
        root_id: String,
    }

    fn setup() -> TestContext {
        let docs_dir = tempdir().expect("docs temp dir");
        let store_dir = tempdir().expect("store temp dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");
        let saved = save_settings(
            &store,
            &DocsSettings {
                root_dir: None,
                roots: vec![DocsRoot {
                    id: String::new(),
                    path: docs_dir.path().to_string_lossy().to_string(),
                }],
            },
        )
        .expect("settings save");
        let root_id = saved.roots[0].id.clone();
        assert!(!root_id.is_empty());
        TestContext { docs: docs_dir, _store_dir: store_dir, store, root_id }
    }

    fn write_docs_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn root_ids_are_deterministic_per_path() {
        assert_eq!(root_id_for("/tmp/docs"), root_id_for("/tmp/docs"));
        assert_ne!(root_id_for("/tmp/docs"), root_id_for("/tmp/other"));
    }

    #[test]
    fn settings_roundtrip_and_validation() {
        let context = setup();
        let read_back = get_settings(&context.store);
        assert_eq!(read_back.roots.len(), 1);
        assert_eq!(read_back.roots[0].id, context.root_id);
        assert_eq!(read_back.root_dir, None);

        let bad = save_settings(
            &context.store,
            &DocsSettings {
                root_dir: None,
                roots: vec![DocsRoot { id: String::new(), path: "relative/path".into() }],
            },
        );
        assert!(bad.is_err());

        let missing = save_settings(
            &context.store,
            &DocsSettings {
                root_dir: None,
                roots: vec![DocsRoot { id: String::new(), path: "/definitely/not/here".into() }],
            },
        );
        assert!(missing.is_err());

        let cleared = save_settings(&context.store, &DocsSettings { root_dir: None, roots: Vec::new() })
            .expect("empty clears");
        assert!(cleared.roots.is_empty());
    }

    #[test]
    fn legacy_root_dir_folds_into_roots_on_read() {
        let docs_dir = tempdir().expect("docs temp dir");
        let store_dir = tempdir().expect("store temp dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");
        let legacy = docs_dir.path().to_string_lossy().to_string();
        store
            .write(
                DOCS_SETTINGS_FILENAME,
                &DocsSettings { root_dir: Some(legacy.clone()), roots: Vec::new() },
            )
            .expect("write legacy settings");

        let settings = get_settings(&store);
        assert_eq!(settings.roots.len(), 1);
        assert_eq!(settings.roots[0].path, legacy);
        assert_eq!(settings.roots[0].id, root_id_for(&legacy));

        // 同一路径两次读取 id 一致，前端链接因此稳定
        assert_eq!(get_settings(&store).roots[0].id, root_id_for(&legacy));
    }

    #[test]
    fn save_settings_dedupes_repeated_paths() {
        let docs_dir = tempdir().expect("docs temp dir");
        let store_dir = tempdir().expect("store temp dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");
        let dir = docs_dir.path().to_string_lossy().to_string();

        let saved = save_settings(
            &store,
            &DocsSettings {
                root_dir: None,
                roots: vec![
                    DocsRoot { id: String::new(), path: dir.clone() },
                    DocsRoot { id: String::new(), path: format!("{dir}/") },
                ],
            },
        )
        .expect("save");
        assert_eq!(saved.roots.len(), 1);
    }

    #[test]
    fn list_entries_orders_dirs_first_and_filters_junk() {
        let context = setup();
        let root = context.docs.path();

        write_docs_file(root, "周报/2026-10.md", "# 周报");
        write_docs_file(root, "readme.md", "hello");
        write_docs_file(root, ".hidden.md", "x");
        write_docs_file(root, "notes.txt", "not markdown");
        fs::create_dir_all(root.join(".git")).expect("git dir");
        fs::write(root.join(".git/config"), "x").expect("git file");

        let entries = list_entries(&context.store, &context.root_id).expect("entries");

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
        let context = setup();
        write_docs_file(context.docs.path(), "big.md", &"x".repeat(3 * 1024 * 1024));

        assert!(read_file(&context.store, &context.root_id, "big.md").is_err());
        assert!(read_file(&context.store, &context.root_id, "../outside.md").is_err());
        assert!(read_file(&context.store, &context.root_id, "missing.md").is_err());
    }

    #[test]
    fn operations_fail_with_unknown_root_id() {
        let context = setup();
        assert!(list_entries(&context.store, "no-such-root").is_err());
        assert!(read_file(&context.store, "no-such-root", "a.md").is_err());
        assert!(save_file(&context.store, "no-such-root", "a.md", "x").is_err());
        assert!(search(&context.store, "no-such-root", "x").is_err());
    }

    #[test]
    fn create_file_appends_md_and_rejects_collisions() {
        let context = setup();
        let root = context.docs.path();
        write_docs_file(root, "note.md", "old");

        let created = create_file(&context.store, &context.root_id, "", "新文档").expect("create");
        assert_eq!(created.name, "新文档.md");
        assert_eq!(created.path, "新文档.md");
        assert!(root.join("新文档.md").exists());

        assert!(create_file(&context.store, &context.root_id, "", "note").is_err()); // 补 .md 后与 note.md 冲突
        assert!(create_file(&context.store, &context.root_id, "", "../逃逸").is_err()); // 名称带分隔符被拒
        assert!(create_file(&context.store, &context.root_id, "", ".隐藏").is_err());
    }

    #[test]
    fn save_and_rename_and_delete_roundtrip() {
        let context = setup();
        let root = context.docs.path();
        let entry = create_file(&context.store, &context.root_id, "", "草稿").expect("create");

        let saved = save_file(&context.store, &context.root_id, &entry.path, "# 标题\n内容").expect("save");
        assert_eq!(saved.size, "# 标题\n内容".len() as u64);
        assert_eq!(read_file(&context.store, &context.root_id, "草稿.md").expect("read"), "# 标题\n内容");

        let renamed = rename_entry(&context.store, &context.root_id, "草稿.md", "正式文档").expect("rename");
        assert_eq!(renamed.path, "正式文档.md");
        assert!(!root.join("草稿.md").exists());
        assert_eq!(read_file(&context.store, &context.root_id, "正式文档.md").expect("read again"), "# 标题\n内容");

        assert_eq!(
            delete_entry(&context.store, &context.root_id, "正式文档.md").expect("delete"),
            "正式文档.md"
        );
        assert!(!root.join("正式文档.md").exists());
    }

    #[test]
    fn rename_file_keeps_md_suffix_and_blocks_collision() {
        let context = setup();
        let root = context.docs.path();
        write_docs_file(root, "a.md", "a");
        write_docs_file(root, "b.md", "b");

        let renamed = rename_entry(&context.store, &context.root_id, "a.md", "新名字").expect("rename");
        assert_eq!(renamed.name, "新名字.md");

        assert!(rename_entry(&context.store, &context.root_id, "新名字.md", "b").is_err()); // 会撞 b.md
        assert!(rename_entry(&context.store, &context.root_id, "missing.md", "x").is_err());
    }

    #[test]
    fn delete_directory_requires_empty() {
        let context = setup();
        let root = context.docs.path();
        write_docs_file(root, "folder/inner.md", "x");

        assert!(delete_entry(&context.store, &context.root_id, "folder").is_err());
        fs::remove_file(root.join("folder/inner.md")).expect("clear");
        assert!(delete_entry(&context.store, &context.root_id, "folder").is_ok());
    }

    #[test]
    fn search_finds_lines_case_insensitively() {
        let context = setup();
        let root = context.docs.path();
        write_docs_file(root, "notes/api.md", "# API\n调用的接口是 sendEmail\n");
        write_docs_file(root, "notes/other.md", "无关内容");

        let results = search(&context.store, &context.root_id, "sendemail").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "notes/api.md");
        assert_eq!(results[0].name, "api.md");
        assert_eq!(results[0].line_number, 2);
        assert!(results[0].line_text.contains("sendEmail"));

        assert!(search(&context.store, &context.root_id, "  ").expect("blank").is_empty());
    }

    #[test]
    fn operations_fail_without_root() {
        let store_dir = tempdir().expect("store dir");
        let store = LocalJsonStore::new(store_dir.path().to_path_buf()).expect("store");

        assert!(list_entries(&store, "any").is_err());
        assert!(read_file(&store, "any", "a.md").is_err());
        assert!(save_file(&store, "any", "a.md", "x").is_err());
    }
}

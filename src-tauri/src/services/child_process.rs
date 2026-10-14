//! 子进程拉起的公共环境处理。macOS 从 Finder/启动台启动的打包应用只带
//! 极简 PATH（/usr/bin:/bin:/usr/sbin:/sbin），找不到 Homebrew、官方安装包
//! 装的命令（go、git 等）；开发模式从终端启动继承完整 PATH，掩盖了问题。
//! 这里统一按「当前 PATH + 常见安装目录」解析程序，并把增强后的 PATH
//! 传给子进程（go run、git 内部还会拉起别的工具）。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// 常见命令安装目录，追加到当前 PATH 之后（用户自己的 PATH 优先）
const COMMON_TOOL_DIRS: &[&str] = &[
    "/opt/homebrew/bin", // Homebrew（Apple Silicon）
    "/usr/local/bin",    // Homebrew（Intel）/ 手动安装
    "/usr/local/go/bin", // Go 官方安装包
];

/// 当前 PATH 追加常见安装目录（已存在的不重复加）
pub fn augmented_path() -> OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&current).collect();
    for dir in COMMON_TOOL_DIRS {
        let dir = Path::new(dir);
        if !dirs.iter().any(|existing| existing.as_path() == dir) {
            dirs.push(dir.to_path_buf());
        }
    }
    std::env::join_paths(dirs).unwrap_or(current)
}

/// 在给定 PATH 里把程序名解析成绝对路径，None 表示没找到。
/// 程序自带路径分隔符时按原样使用（相对/绝对路径都交给调用方）。
pub fn resolve_program(program: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    if program.contains('/') || program.contains('\\') {
        return Some(PathBuf::from(program));
    }
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// 解析程序并构造 tokio Command，子进程 PATH 一并设为增强后的值。
/// 第二个返回值表示是否解析成功；失败时仍按原名构造，让 spawn 自行报错。
pub fn tokio_command(program: &str) -> (tokio::process::Command, bool) {
    let path = augmented_path();
    let resolved = resolve_program(program, &path);
    let found = resolved.is_some();
    let mut command =
        tokio::process::Command::new(resolved.unwrap_or_else(|| PathBuf::from(program)));
    command.env("PATH", path);
    (command, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn augmented_path_appends_common_dirs_and_keeps_existing() {
        let original: Vec<PathBuf> =
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();

        let dirs: Vec<PathBuf> = std::env::split_paths(&augmented_path()).collect();

        for dir in COMMON_TOOL_DIRS {
            assert!(
                dirs.iter().any(|d| d.as_os_str() == *dir),
                "missing common dir {dir}"
            );
        }
        // 原 PATH 条目仍在且顺序不变（用户配置优先）
        for (index, dir) in original.iter().enumerate() {
            assert_eq!(dirs.get(index), Some(dir));
        }
    }

    #[test]
    fn resolve_program_prefers_executable_on_path_and_falls_back() {
        let dir = std::env::temp_dir().join(format!("eidolon-child-{}", std::process::id()));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("create bin dir");

        let fake_go = bin.join("go");
        std::fs::write(&fake_go, "#!/bin/sh\n").expect("write fake go");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake_go, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }

        let path = std::env::join_paths([Path::new("/nonexistent-eidolon-dir"), bin.as_path()])
            .expect("join path");

        assert_eq!(resolve_program("go", &path), Some(fake_go.clone()));
        // 自带路径分隔符的程序不参与查找
        assert_eq!(
            resolve_program("./cmd/go", &path),
            Some(PathBuf::from("./cmd/go"))
        );
        assert_eq!(resolve_program("definitely-not-a-tool", &path), None);

        #[cfg(unix)]
        {
            // 没有可执行位的文件不能当选
            use std::os::unix::fs::PermissionsExt;
            let plain = bin.join("plain");
            std::fs::write(&plain, "x").expect("write plain");
            std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644))
                .expect("chmod");
            assert_eq!(resolve_program("plain", &path), None);
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 打包版应用从 Finder 启动只有极简 PATH，go 只能靠 COMMON_TOOL_DIRS 兜底找到
    #[test]
    fn resolves_go_from_common_dirs_under_minimal_path() {
        let minimal = std::env::join_paths(["/usr/bin", "/bin", "/usr/sbin", "/sbin"])
            .expect("join minimal path");

        let Some(found) = resolve_program("go", &minimal) else {
            // 本机没装 Go 时无从断言，跳过而不是伪造通过
            return;
        };
        let from_common_dir = COMMON_TOOL_DIRS
            .iter()
            .any(|dir| found.starts_with(Path::new(dir)));
        assert!(
            from_common_dir,
            "极简 PATH 下解析到的 {} 不在常见安装目录内",
            found.display()
        );
    }
}

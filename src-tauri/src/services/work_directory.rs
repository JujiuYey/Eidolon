use std::ffi::OsString;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR_STR};

pub fn normalize_work_directory(work_directory: &str) -> Result<PathBuf, String> {
    let trimmed = work_directory.trim();
    if trimmed.is_empty() {
        return Err("work_directory 不能为空".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("work_directory 必须是绝对路径".to_string());
    }

    normalize_absolute_path(&path)
}

pub fn assert_path_within_work_directory(
    work_directory: &str,
    requested_path: &Path,
) -> Result<PathBuf, String> {
    let root = normalize_work_directory(work_directory)?;

    let candidate = if requested_path.as_os_str().is_empty() {
        root.clone()
    } else if requested_path.is_absolute() {
        normalize_absolute_path(requested_path)?
    } else {
        normalize_absolute_path(root.join(requested_path))?
    };

    if candidate.starts_with(&root) {
        Ok(candidate)
    } else {
        Err(format!(
            "路径超出工作目录边界: {}",
            requested_path.display()
        ))
    }
}

fn normalize_absolute_path(path: impl AsRef<Path>) -> Result<PathBuf, String> {
    let path = path.as_ref();
    if !path.is_absolute() {
        return Err("路径必须是绝对路径".to_string());
    }

    let mut prefix: Option<OsString> = None;
    let mut has_root = false;
    let mut segments: Vec<OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(value) => {
                prefix = Some(value.as_os_str().to_os_string());
            }
            Component::RootDir => {
                has_root = true;
            }
            Component::CurDir => {}
            Component::Normal(value) => {
                segments.push(value.to_os_string());
            }
            Component::ParentDir => {
                if !segments.is_empty() {
                    segments.pop();
                }
            }
        }
    }

    let mut normalized = PathBuf::new();

    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }

    if has_root {
        normalized.push(MAIN_SEPARATOR_STR);
    }

    for segment in segments {
        normalized.push(segment);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{assert_path_within_work_directory, normalize_work_directory};

    #[test]
    fn accepts_path_inside_work_directory() {
        let normalized =
            assert_path_within_work_directory("/tmp/eidolon", Path::new("notes/todo.md"))
                .expect("path inside work directory should be accepted");

        assert_eq!(normalized, Path::new("/tmp/eidolon/notes/todo.md"));
    }

    #[test]
    fn accepts_nested_child_path() {
        let normalized = assert_path_within_work_directory(
            "/tmp/eidolon",
            Path::new("src/./components/../components/app.ts"),
        )
        .expect("nested child path should be accepted");

        assert_eq!(normalized, Path::new("/tmp/eidolon/src/components/app.ts"));
    }

    #[test]
    fn rejects_parent_traversal_outside_work_directory() {
        let error = assert_path_within_work_directory("/tmp/eidolon", Path::new("../outside.txt"))
            .expect_err("escaped path should be rejected");

        assert!(error.contains("路径超出工作目录边界"));
    }

    #[test]
    fn rejects_absolute_path_outside_work_directory() {
        let error = assert_path_within_work_directory("/tmp/eidolon", Path::new("/etc/hosts"))
            .expect_err("absolute path outside work directory should be rejected");

        assert!(error.contains("路径超出工作目录边界"));
    }

    #[test]
    fn rejects_empty_work_directory() {
        let error = normalize_work_directory("   ").expect_err("blank work directory should fail");

        assert_eq!(error, "work_directory 不能为空");
    }
}

//! 代码接口清单导出：按框架类型在后端仓库目录里执行该框架的
//! 导出命令，捕获 stdout 并校验为合法 JSON 后原样返回。
//!
//! 框架相关的只有命令模板（当前支持 vef-go：`VEF_EXPORT_API=- go
//! run ./cmd/server`）；解析清单、映射为请求定义由前端适配器完成。
//! 长任务支持取消：每个导出任务把子进程注册进全局表，取消时 kill。

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;

/// 导出超时上限。`go run` 首次编译可能耗时较长，留足余量
const EXPORT_TIMEOUT: Duration = Duration::from_secs(600);

/// stderr 报错时保留的尾部长度
const STDERR_TAIL_BYTES: usize = 1500;

/// 运行中的导出任务：export_id -> 子进程
fn children() -> &'static Mutex<HashMap<String, Arc<AsyncMutex<Child>>>> {
    static CHILDREN: OnceLock<Mutex<HashMap<String, Arc<AsyncMutex<Child>>>>> = OnceLock::new();
    CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 已取消的导出任务标记，用于把 kill 引起的非零退出翻译为「已取消」
fn cancelled() -> &'static Mutex<HashSet<String>> {
    static CANCELLED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CANCELLED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// 框架 -> 导出命令（程序与参数）。新增框架时在此登记。
fn export_command(framework: &str) -> Option<(&'static str, Vec<&'static str>)> {
    match framework {
        "vef-go" => Some(("go", vec!["run", "./cmd/server"])),
        _ => None,
    }
}

/// 框架导出需要注入的环境变量
fn export_env(framework: &str) -> Vec<(&'static str, &'static str)> {
    match framework {
        "vef-go" => vec![("VEF_EXPORT_API", "-")],
        _ => Vec::new(),
    }
}

pub fn supported_frameworks() -> Vec<&'static str> {
    vec!["vef-go", "vef-java", "dhyx-java"]
}

/// 在仓库目录执行框架导出命令，返回清单 JSON 文本。
/// stdout 与 stderr 并发读取（清单可达 MB 级，串行等待会塞满管道）；
/// 退出码非零或输出不是 JSON 时报错，被取消的任务返回「已取消」。
pub async fn export(framework: &str, repo_path: &str, export_id: &str) -> Result<String, String> {
    let export_id = export_id.trim();
    if export_id.is_empty() {
        return Err("导出任务 ID 不能为空".to_string());
    }

    let repo = Path::new(repo_path);
    if !repo.is_dir() {
        return Err(format!("仓库路径不存在或不是目录: {repo_path}"));
    }

    // Java 系框架：静态扫描源码（只读、毫秒级），不经过子进程与取消注册表
    if framework == "vef-java" || framework == "dhyx-java" {
        return crate::services::code_scan_java::scan(repo);
    }

    let (program, args) = export_command(framework).ok_or_else(|| {
        format!(
            "暂不支持的框架类型 {framework}，当前支持: {}",
            supported_frameworks().join(", ")
        )
    })?;

    let mut command = Command::new(program);
    command
        .args(&args)
        .current_dir(repo)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in export_env(framework) {
        command.env(key, value);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("启动导出命令失败（{program}）: {error}"))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let handle = Arc::new(AsyncMutex::new(child));
    children()
        .lock()
        .map_err(|_| "导出任务表已损坏".to_string())?
        .insert(export_id.to_string(), Arc::clone(&handle));

    let outcome = run_to_completion(Arc::clone(&handle), stdout_pipe, stderr_pipe).await;

    children()
        .lock()
        .map_err(|_| "导出任务表已损坏".to_string())?
        .remove(export_id);

    let was_cancelled = cancelled()
        .lock()
        .map_err(|_| "导出任务表已损坏".to_string())?
        .remove(export_id);

    if was_cancelled {
        return Err("已取消".to_string());
    }

    let (status, stdout, stderr) = outcome?;

    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr).trim_end().to_string();
        let tail = if stderr_text.len() > STDERR_TAIL_BYTES {
            stderr_text[stderr_text.len() - STDERR_TAIL_BYTES..].to_string()
        } else {
            stderr_text
        };
        return Err(format!("导出命令退出码 {:?}：{}", status.code(), tail));
    }

    let stdout_text = String::from_utf8_lossy(&stdout).trim().to_string();
    if stdout_text.is_empty() {
        return Err("导出命令没有输出清单，请确认框架版本支持清单导出".to_string());
    }

    serde_json::from_str::<serde_json::Value>(&stdout_text)
        .map_err(|error| format!("导出输出不是合法 JSON: {error}"))?;

    Ok(stdout_text)
}

/// 等待进程退出并并发收集输出，整体受超时约束
async fn run_to_completion(
    handle: Arc<AsyncMutex<Child>>,
    stdout_pipe: Option<tokio::process::ChildStdout>,
    stderr_pipe: Option<tokio::process::ChildStderr>,
) -> Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>), String> {
    let wait_task = async {
        let mut child = handle.lock().await;
        child.wait().await
    };

    let stdout_task = read_pipe(stdout_pipe);
    let stderr_task = read_pipe(stderr_pipe);

    let joined = tokio::time::timeout(EXPORT_TIMEOUT, async {
        tokio::join!(wait_task, stdout_task, stderr_task)
    })
    .await;

    match joined {
        Ok((status, stdout, stderr)) => {
            let status = status.map_err(|error| format!("等待导出进程失败: {error}"))?;
            Ok((status, stdout, stderr))
        }
        Err(_) => {
            // 超时：kill_on_drop 在锁释放后回收子进程，这里主动 kill 加速退出
            if let Ok(mut child) = handle.try_lock() {
                let _ = child.start_kill();
            }
            Err(format!(
                "导出超时（超过 {} 秒）。冷缓存首次编译较慢，可重试一次。",
                EXPORT_TIMEOUT.as_secs()
            ))
        }
    }
}

async fn read_pipe<P: tokio::io::AsyncRead + Unpin>(pipe: Option<P>) -> Vec<u8> {
    let mut buffer = Vec::new();
    if let Some(mut pipe) = pipe {
        // 读端被对端关闭（含 kill）时返回已读内容
        let _ = pipe.read_to_end(&mut buffer).await;
    }
    buffer
}

/// 取消导出任务。进程可能已经结束，返回是否找到了还在运行的任务。
pub async fn cancel(export_id: &str) -> Result<bool, String> {
    let handle = children()
        .lock()
        .map_err(|_| "导出任务表已损坏".to_string())?
        .get(export_id)
        .cloned();

    let Some(handle) = handle else {
        return Ok(false);
    };

    cancelled()
        .lock()
        .map_err(|_| "导出任务表已损坏".to_string())?
        .insert(export_id.to_string());

    let mut child = handle.lock().await;
    match child.start_kill() {
        Ok(()) => Ok(true),
        // 进程刚好已退出
        Err(error) if error.kind() == ErrorKind::InvalidInput => Ok(true),
        Err(error) => Err(format!("终止导出进程失败: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_command_table_only_knows_vef_go() {
        assert_eq!(
            export_command("vef-go").expect("vef-go"),
            ("go", vec!["run", "./cmd/server"])
        );
        assert!(export_command("spring-boot").is_none());
        assert_eq!(export_env("vef-go"), vec![("VEF_EXPORT_API", "-")]);
    }

    #[tokio::test]
    async fn export_rejects_missing_repo_and_unknown_framework() {
        let error = export("vef-go", "/nonexistent/path/for/eidolon", "t1")
            .await
            .expect_err("missing repo must be rejected");
        assert!(error.contains("不是目录"), "got {error}");

        let error = export("ruby", "/tmp", "t2")
            .await
            .expect_err("unknown framework must be rejected");
        assert!(error.contains("暂不支持的框架类型"), "got {error}");
    }

    #[tokio::test]
    async fn export_via_shell_fakes_go_happy_path() {
        // 用一个临时目录 + 假 `go` 脚本冒充 vef 导出器，走完整导出路径：
        // spawn、并发读输出、JSON 校验、注册表清理。
        let dir = std::env::temp_dir().join(format!("eidolon-export-{}", std::process::id()));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("create bin dir");

        let fake_go = bin.join("go");
        std::fs::write(
            &fake_go,
            "#!/bin/sh\nif [ \"$1\" = \"run\" ]; then echo '{\"framework\":\"vef\"}'; exit 0; fi\nexit 1\n",
        )
        .expect("write fake go");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake_go, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }

        // export_command 固定用 PATH 上的 go；把假 bin 前置到 PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));

        let result = export("vef-go", &dir.to_string_lossy(), "t3").await;
        std::env::set_var("PATH", old_path);

        let manifest = result.expect("fake go export should succeed");
        assert!(manifest.contains("\"vef\""));
        assert!(children().lock().expect("registry").get("t3").is_none());
    }
}

use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
pub struct AppPaths {
    pub app_data: String,
    pub app_log: String,
}

/// 获取应用数据目录和应用日志目录
#[tauri::command]
pub fn get_app_paths(app: AppHandle) -> Result<AppPaths, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let app_log = app_data.join("logs");

    Ok(AppPaths {
        app_data: app_data.to_string_lossy().to_string(),
        app_log: app_log.to_string_lossy().to_string(),
    })
}

/// 打开指定路径的目录
#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 在系统默认浏览器中打开链接。只放行 http/https，
/// 其余 scheme（file://、自定义协议等）一律拒绝。
#[tauri::command]
pub fn open_external_url(raw_url: String) -> Result<(), String> {
    let parsed = url::Url::parse(raw_url.trim()).map_err(|error| format!("无效的链接地址: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("仅支持 http/https 链接".to_string());
    }
    let target = parsed.as_str().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

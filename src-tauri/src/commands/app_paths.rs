use tauri::{AppHandle, Manager};
use serde::Serialize;

#[derive(Serialize)]
pub struct AppPaths {
    pub app_data: String,
    pub app_log: String,
}

/// 获取应用数据目录和应用日志目录
#[tauri::command]
pub fn get_app_paths(app: AppHandle) -> Result<AppPaths, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

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

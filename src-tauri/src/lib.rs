mod commands;
mod db;
pub mod models;
pub mod services;
use db::LocalJsonStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_paths::get_app_paths,
            commands::app_paths::open_directory,
            commands::model_config::list_provider_configs,
            commands::model_config::upsert_provider_config,
            commands::model_config::delete_provider_config,
            commands::model_config::fetch_provider_models,
            commands::test_connection::test_ai_connection,
            commands::codegen::generate_crud,
            commands::codegen::parse_sql_ddl,
            commands::codegen::generate_go_crud,
        ])
        .setup(|app| {
            // 窗口启动时自动最大化
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.maximize();
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // 获取应用数据目录
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;

            // 创建本地 JSON 存储
            let store =
                LocalJsonStore::new(app_data_dir).map_err(|error| std::io::Error::other(error))?;

            log::info!("数据存储位置: {}", store.data_dir().display());

            app.manage(store);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

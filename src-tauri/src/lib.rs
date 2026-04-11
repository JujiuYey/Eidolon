mod commands;
mod db;
pub mod models;
pub mod services;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::model_config::list_model_configs,
            commands::model_config::create_model_config,
            commands::model_config::update_model_config,
            commands::model_config::delete_model_config,
            commands::model_config::set_default_config,
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

            let mongo = tauri::async_runtime::block_on(async {
                db::mongodb::AppMongoDb::open().await
            })
            .map_err(std::io::Error::other)?;

            app.manage(mongo);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

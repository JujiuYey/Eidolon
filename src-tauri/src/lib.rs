mod commands;
mod db;
pub mod models;
pub mod services;
use db::{ApiClientDatabase, LocalJsonStore};
use services::api_http::ApiHttpClient;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // Agent conversation commands
            commands::agent_conversation::list_agent_conversations,
            commands::agent_conversation::list_recent_agent_conversations,
            commands::agent_conversation::create_agent_conversation,
            commands::agent_conversation::get_agent_conversation,
            commands::agent_conversation::delete_agent_conversation,
            commands::agent_conversation::list_agent_conversation_messages,
            commands::agent_conversation::send_agent_conversation_message,
            // Agent profile commands
            commands::agent_profile::list_agent_profiles,
            commands::agent_profile::get_agent_profile,
            commands::agent_profile::upsert_agent_profile,
            commands::agent_profile::delete_agent_profile,
            // App paths commands
            commands::app_paths::get_app_paths,
            commands::app_paths::open_directory,
            // Conversation commands
            commands::conversation::send_conversation_message,
            // Default model commands
            commands::default_model::list_default_model_settings,
            commands::default_model::upsert_default_model_setting,
            // MCP service commands
            commands::mcp_service::list_mcp_services,
            commands::mcp_service::upsert_mcp_service,
            commands::mcp_service::delete_mcp_service,
            commands::mcp_service::discover_mcp_service,
            // Model config commands
            commands::model_config::list_provider_settings,
            commands::model_config::upsert_provider_setting,
            commands::model_config::delete_provider_setting,
            commands::model_config::list_provider_models,
            commands::model_config::replace_provider_models,
            commands::model_config::delete_provider_models,
            // Test connection command
            commands::test_connection::test_ai_connection,
            // Codegen commands
            commands::codegen::generate_crud,
            commands::codegen::parse_sql_ddl,
            commands::codegen::generate_go_crud,
            // API client commands
            commands::api_client::list_api_projects,
            commands::api_client::create_api_project,
            commands::api_client::rename_api_project,
            commands::api_client::update_api_project,
            commands::api_client::preview_api_project_deletion,
            commands::api_client::delete_api_project,
            commands::api_client::list_api_groups,
            commands::api_client::create_api_group,
            commands::api_client::rename_api_group,
            commands::api_client::delete_api_group,
            commands::api_client::reorder_api_groups,
            commands::api_client::list_api_requests,
            commands::api_client::get_api_request,
            commands::api_client::create_api_request,
            commands::api_client::update_api_request,
            commands::api_client::duplicate_api_request,
            commands::api_client::move_api_request,
            commands::api_client::delete_api_request,
            commands::api_client::reorder_api_requests,
            commands::api_client::list_api_environments,
            commands::api_client::upsert_api_environment,
            commands::api_client::delete_api_environment,
            commands::api_client::list_api_request_histories,
            commands::api_client::get_api_request_history,
            commands::api_client::clear_api_request_histories,
            commands::api_request::send_api_request,
            commands::api_request::cancel_api_request,
            commands::api_request::preview_api_request,
            commands::api_generate::generate_api_request_body,
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

            // 创建本地 JSON 存储（消耗 app_data_dir）
            let store = LocalJsonStore::new(app_data_dir.clone())
                .map_err(|error| std::io::Error::other(error))?;

            log::info!("数据存储位置: {}", store.data_dir().display());

            app.manage(store);

            // 初始化接口请求工具专用 SQLite 数据库与 HTTP 客户端
            let api_database = ApiClientDatabase::open(&app_data_dir)
                .map_err(|error| std::io::Error::other(error))?;
            log::info!("接口请求数据库位置: {}", api_database.path().display());
            app.manage(api_database);

            let http_client = ApiHttpClient::new().map_err(|error| std::io::Error::other(error))?;
            app.manage(http_client);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

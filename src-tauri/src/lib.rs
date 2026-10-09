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
            // PM (产品经理分身) commands
            commands::pm::list_pm_conversations,
            commands::pm::create_pm_conversation,
            commands::pm::rename_pm_conversation,
            commands::pm::delete_pm_conversation,
            commands::pm::list_pm_conversation_messages,
            commands::pm::send_pm_message,
            commands::pm::list_pm_skills,
            commands::pm::get_pm_skill_content,
            commands::pm::get_pm_settings,
            commands::pm::upsert_pm_settings,
            // Test connection command
            commands::test_connection::test_ai_connection,
            // API client commands
            commands::api_client::list_api_projects,
            commands::api_client::get_api_project,
            commands::api_client::create_api_project,
            commands::api_client::update_api_project,
            commands::api_client::get_api_project_deletion_impact,
            commands::api_client::delete_api_project,
            commands::api_client::list_api_groups,
            commands::api_client::create_api_group,
            commands::api_client::rename_api_group,
            commands::api_client::move_api_group,
            commands::api_client::delete_api_group,
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
            // Email commands
            commands::email::list_smtp_accounts,
            commands::email::upsert_smtp_account,
            commands::email::delete_smtp_account,
            commands::email::list_mail_templates,
            commands::email::upsert_mail_template,
            commands::email::delete_mail_template,
            commands::email::send_email,
            commands::email::test_smtp_connection,
            commands::email::list_sent_emails,
            commands::email::delete_sent_email,
            commands::email::clear_sent_emails,
            commands::api_generate::generate_api_request_body,
            // Weekly report commands
            commands::weekly_report::list_weekly_report_repos,
            commands::weekly_report::add_weekly_report_repo,
            commands::weekly_report::remove_weekly_report_repo,
            commands::weekly_report::fetch_weekly_report_commits,
            commands::weekly_report::list_weekly_reports,
            commands::weekly_report::save_weekly_report,
            commands::weekly_report::delete_weekly_report,
            commands::weekly_report::export_weekly_report,
            commands::weekly_report::polish_weekly_report,
            // ZenTao commands
            commands::zentao::list_zentao_accounts,
            commands::zentao::upsert_zentao_account,
            commands::zentao::delete_zentao_account,
            commands::zentao::test_zentao_connection,
            commands::zentao::fetch_zentao_my_work,
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

            // 禅道客户端（HTTP + token 缓存）
            let zentao_client = services::zentao::ZentaoClient::new()
                .map_err(|error| std::io::Error::other(error))?;
            app.manage(zentao_client);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

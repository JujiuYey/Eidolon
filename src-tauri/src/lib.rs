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
            commands::mcp_server::get_mcp_server_info,
            commands::mcp_server::update_mcp_server_settings,
            commands::mcp_server::regenerate_mcp_server_token,
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
            // Docs (文档管理) commands
            commands::docs::get_docs_settings,
            commands::docs::upsert_docs_settings,
            commands::docs::list_docs_entries,
            commands::docs::read_docs_file,
            commands::docs::save_docs_file,
            commands::docs::create_docs_file,
            commands::docs::create_docs_directory,
            commands::docs::rename_docs_entry,
            commands::docs::delete_docs_entry,
            commands::docs::search_docs,
            commands::docs_ai::chat_docs_ai,
            // Test connection command
            commands::test_connection::test_ai_connection,
            // API client commands
            commands::api_client::list_api_projects,
            commands::api_client::get_api_project,
            commands::api_client::get_api_client_stats,
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
            commands::code_import::export_code_manifest,
            commands::code_import::cancel_code_export,
            commands::code_import::list_api_import_sources,
            commands::code_import::run_api_import,
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
            // Git account commands
            commands::git::list_git_accounts,
            commands::git::upsert_git_account,
            commands::git::delete_git_account,
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
            // Task manager commands
            commands::task::list_tasks,
            commands::task::upsert_task,
            commands::task::delete_task,
            commands::task::parse_task_text,
            commands::task::suggest_task_plan,
            // Schema dict (数据字典) commands
            commands::schema_dict::list_schema_databases,
            commands::schema_dict::upsert_schema_database,
            commands::schema_dict::delete_schema_database,
            commands::schema_dict::list_schema_snapshots,
            commands::schema_dict::delete_schema_snapshot,
            commands::schema_dict::parse_schema_input,
            commands::schema_dict::import_schema_snapshot,
            commands::schema_dict::get_schema_timeline,
            commands::schema_dict::interpret_schema_changes,
            commands::schema_dict::ask_schema_dict,
            commands::schema_dict::call_mcp_tool,
            // Collect (收藏夹) commands
            commands::collect::list_collect_items,
            commands::collect::upsert_collect_item,
            commands::collect::delete_collect_item,
            commands::collect::fetch_link_metadata,
            commands::collect::summarize_collect_link,
            commands::collect::ask_collect_items,
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

            // 对外 MCP 服务（设置页可开关；启用时随应用启动）
            let mcp_handle = services::mcp_server::McpServerHandle::default();
            app.manage(mcp_handle);
            {
                let mcp_store = std::sync::Arc::new(
                    LocalJsonStore::new(app_data_dir.clone())
                        .map_err(|error| std::io::Error::other(error))?,
                );
                match services::mcp_server::load_or_init(&mcp_store) {
                    Ok(settings) if settings.enabled => {
                        let app_handle = app.handle().clone();
                        tauri::async_runtime::spawn(async move {
                            let handle =
                                app_handle.state::<services::mcp_server::McpServerHandle>();
                            match handle.start(&settings, mcp_store).await {
                                Ok(port) => log::info!("MCP 服务随应用启动: {port}"),
                                Err(error) => log::error!("MCP 服务启动失败: {error}"),
                            }
                        });
                    }
                    Ok(_) => {}
                    Err(error) => log::error!("读取 MCP 服务设置失败: {error}"),
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

//! 对外 MCP 服务：把 Eidolon 的文档与个人任务能力以 MCP 工具的形式
//! 暴露给本机其他程序（Claude Code / Cursor / ZCode 等）。
//!
//! - 传输：MCP streamable-http，仅绑定 127.0.0.1；外部程序配置
//!   `http://127.0.0.1:<port>/mcp` 即可连接
//! - 鉴权：访问令牌，客户端通过 `Authorization: Bearer <token>` 携带，
//!   由 axum 中间件在请求进入 MCP 会话前校验
//! - 工具：文档是 `services/docs.rs` 的薄包装，磁盘沙箱与界面操作完全
//!   一致，文档始终落在用户的文档根目录里；任务是 `TaskRepository` 的
//!   薄包装，刻意不提供删除（外部来源只增不删）
//! - 事件：文档写操作（write/edit/delete）成功后向窗口广播
//!   `docs-external-change`，工作区据此刷新文件树、跟进或提示冲突
//!
//! 与 `services/mcp_service.rs`（Eidolon 作为客户端连接外部 MCP）独立。

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo, ToolsCapability},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::db::repositories::task_repo::TaskRepository;
use crate::db::{ApiClientDatabase, LocalJsonStore};
use crate::models::mcp_server::McpServerSettings;
use crate::models::task::{
    normalize_status, Task, TASK_PRIORITY_NONE, TASK_STATUS_DOING, TASK_STATUS_DONE,
    TASK_STATUS_TODO,
};
use crate::services::docs;

/// 设置文件名（LocalJsonStore 存储，不含扩展名）
pub const SETTINGS_FILE: &str = "mcp_server_settings";

/// 端口被占用时向后探测的范围
const PORT_PROBE_LIMIT: u16 = 20;

/// 前端事件名：MCP 写操作改变文档库时广播
pub const DOCS_EXTERNAL_CHANGE_EVENT: &str = "docs-external-change";

/// MCP 写操作导致的文档变更（事件载荷，path 为相对文档库根目录的路径）
#[derive(Clone, Debug, Serialize)]
pub struct DocsExternalChange {
    pub root_id: String,
    pub path: String,
    /// write | edit | delete
    pub action: &'static str,
}

/// 文档变更回调。生产环境用 `docs_change_emitter` 包一层 AppHandle 广播给
/// 窗口；测试注入捕获器断言事件，MCP 模块本身不依赖 Tauri 运行时。
pub type DocsChangeEmitter = Arc<dyn Fn(DocsExternalChange) + Send + Sync>;

// ============================================================
// 设置读写
// ============================================================

pub fn get_settings(store: &LocalJsonStore) -> McpServerSettings {
    store
        .read::<McpServerSettings>(SETTINGS_FILE)
        .unwrap_or_default()
}

/// 保存设置；令牌为空时自动生成（首次启用）
pub fn save_settings(
    store: &LocalJsonStore,
    mut settings: McpServerSettings,
) -> Result<McpServerSettings, String> {
    if settings.token.trim().is_empty() {
        settings.token = generate_token();
    }
    settings.port = settings.port.max(1);
    store
        .write(SETTINGS_FILE, &settings)
        .map_err(|error| format!("保存 MCP 服务设置失败: {error}"))?;
    Ok(settings)
}

/// 读取设置并保证令牌存在（应用启动与设置页共用）
pub fn load_or_init(store: &LocalJsonStore) -> Result<McpServerSettings, String> {
    let settings = get_settings(store);
    if settings.token.trim().is_empty() {
        save_settings(store, settings)
    } else {
        Ok(settings)
    }
}

pub fn generate_token() -> String {
    // 43 字符左右的字母数字令牌，碰撞概率可忽略
    nanoid::nanoid!(43)
}

// ============================================================
// MCP 工具参数（schema 由 rmcp 宏从字段文档注释生成）
// ============================================================

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RootParams {
    /// 文档库 ID，来自 docs_list_roots 返回
    pub root_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PathParams {
    /// 文档库 ID
    pub root_id: String,

    /// 文件相对路径（`/` 分隔，不带开头斜杠，如 `日记/2026-10.md`）
    pub path: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct WriteParams {
    /// 文档库 ID
    pub root_id: String,

    /// 文件相对路径（`/` 分隔，不带开头斜杠）；文件不存在时自动创建（含父目录）
    pub path: String,

    /// 完整的 Markdown 文件内容（整体覆盖保存）
    pub content: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct EditParams {
    /// 文档库 ID
    pub root_id: String,

    /// 文件相对路径（`/` 分隔，不带开头斜杠）；文件必须已存在
    pub path: String,

    /// 要替换的原文片段，必须与文件内容逐字符一致（含缩进与换行）
    pub old_string: String,

    /// 替换后的新内容（传空字符串即删除该片段）
    pub new_string: String,

    /// 匹配到多处时是否全部替换（默认 false：多处匹配直接报错）
    pub replace_all: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// 文档库 ID
    pub root_id: String,

    /// 搜索关键词
    pub keyword: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskListParams {
    /// 可选状态过滤：todo | doing | done；留空或 all 返回全部
    pub status: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskCreateParams {
    /// 任务标题（必填）
    pub title: String,

    /// 补充备注（可选，纯文本）
    pub notes: Option<String>,

    /// 优先级：high | medium | low | none（可选，默认 none）
    pub priority: Option<String>,

    /// 截止日期 YYYY-MM-DD（可选；格式不合法时忽略）
    pub due_date: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskUpdateParams {
    /// 任务 ID，来自 task_list 返回
    pub task_id: String,

    /// 新标题（不传保持不变）
    pub title: Option<String>,

    /// 新备注（传空字符串清除）
    pub notes: Option<String>,

    /// 新优先级：high | medium | low | none（不传保持不变）
    pub priority: Option<String>,

    /// 新截止日期 YYYY-MM-DD（传空字符串清除；不传保持不变）
    pub due_date: Option<String>,

    /// 新状态：todo | doing | done（不传保持不变）
    pub status: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskSetStatusParams {
    /// 任务 ID，来自 task_list 返回
    pub task_id: String,

    /// 目标状态：todo（待办）| doing（进行中）| done（完成）
    pub status: String,
}

// ============================================================
// MCP 服务端实现
// ============================================================

/// Eidolon MCP 服务端。工具是 `services/docs.rs` 与 `TaskRepository`
/// 的薄包装：磁盘沙箱、状态机（完成时间补记/清除）与界面操作共用。
/// 刻意不暴露任务删除——外部来源只增改，不删。
#[derive(Clone)]
pub struct EidolonMcpServer {
    store: Arc<LocalJsonStore>,
    database: Arc<ApiClientDatabase>,
    emit_docs_change: DocsChangeEmitter,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl EidolonMcpServer {
    pub fn new(
        store: Arc<LocalJsonStore>,
        database: Arc<ApiClientDatabase>,
        emit_docs_change: DocsChangeEmitter,
    ) -> Self {
        Self {
            store,
            database,
            emit_docs_change,
            tool_router: Self::tool_router(),
        }
    }

    fn into_error(error: String) -> McpError {
        McpError::internal_error(error, None)
    }

    /// 文档写操作成功后通知前端（工作区据此刷新文件树/提示冲突）
    fn notify_docs_change(&self, root_id: &str, path: &str, action: &'static str) {
        (self.emit_docs_change)(DocsExternalChange {
            root_id: root_id.to_string(),
            path: path.to_string(),
            action,
        });
    }

    /// 文档服务要求相对路径不带开头斜杠；外部客户端习惯给 `/a/b`，这里归一化
    fn relative(path: &str) -> String {
        path.trim_start_matches('/').to_string()
    }

    /// 按 id 取任务；找不到时返回客户端参数错误
    fn find_task(repo: &TaskRepository<'_>, task_id: &str) -> Result<Task, McpError> {
        let id = task_id.trim();
        if id.is_empty() {
            return Err(McpError::invalid_params("task_id 不能为空", None));
        }
        repo.list()
            .map_err(Self::into_error)?
            .into_iter()
            .find(|task| task.id == id)
            .ok_or_else(|| McpError::invalid_params(format!("未找到 id 为 {id} 的任务"), None))
    }

    /// 列出所有文档库（根目录）
    #[tool(
        name = "docs_list_roots",
        description = "列出 Eidolon 的所有文档库（根目录）。返回每個文档库的 id 与磁盘路径；后续工具都用 id 指定文档库。"
    )]
    async fn docs_list_roots(&self) -> Result<String, McpError> {
        let settings = docs::get_settings(&self.store);
        let roots: Vec<_> = settings
            .roots
            .iter()
            .map(|root| serde_json::json!({ "id": root.id, "path": root.path }))
            .collect();
        serde_json::to_string_pretty(&roots).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 列出文档库中的文件与文件夹
    #[tool(
        name = "docs_list",
        description = "列出指定文档库中的全部文件与文件夹（扁平列表，path 为相对路径，is_dir 区分类型）。用 docs_list_roots 先获取文档库 id。"
    )]
    async fn docs_list(
        &self,
        Parameters(params): Parameters<RootParams>,
    ) -> Result<String, McpError> {
        let entries = docs::list_entries(&self.store, &params.root_id).map_err(Self::into_error)?;
        serde_json::to_string_pretty(&entries).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 读取文档内容
    #[tool(
        name = "docs_read",
        description = "读取 Eidolon 文档库中一个 Markdown 文件的完整内容（上限 2MB）。path 为 docs_list 返回的相对路径。"
    )]
    async fn docs_read(
        &self,
        Parameters(params): Parameters<PathParams>,
    ) -> Result<String, McpError> {
        docs::read_file(&self.store, &params.root_id, &Self::relative(&params.path))
            .map_err(Self::into_error)
    }

    /// 写入文档（不存在则创建）
    #[tool(
        name = "docs_write",
        description = "把完整的 Markdown 内容写入 Eidolon 文档库的指定文件：文件不存在时自动创建（含父目录），存在则整体覆盖。返回保存后的文件信息。"
    )]
    async fn docs_write(
        &self,
        Parameters(params): Parameters<WriteParams>,
    ) -> Result<String, McpError> {
        let path = Self::relative(&params.path);
        let entry = docs::save_file(&self.store, &params.root_id, &path, &params.content)
            .map_err(Self::into_error)?;
        self.notify_docs_change(&params.root_id, &path, "write");
        serde_json::to_string_pretty(&entry).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 局部编辑文档（精确替换，只传改动片段，避免整篇读改写）
    #[tool(
        name = "docs_edit",
        description = "对 Eidolon 文档库中已有文件做局部替换编辑：old_string 必须与文件内容完全一致，默认要求唯一匹配（多处匹配会报错，可传 replace_all=true 全部替换）。局部修改优先用本工具，避免整篇重写；新建文件用 docs_write。返回保存后的文件信息。"
    )]
    async fn docs_edit(
        &self,
        Parameters(params): Parameters<EditParams>,
    ) -> Result<String, McpError> {
        if params.old_string.is_empty() {
            return Err(McpError::invalid_params("old_string 不能为空", None));
        }

        let path = Self::relative(&params.path);
        let content =
            docs::read_file(&self.store, &params.root_id, &path).map_err(Self::into_error)?;

        let count = content.matches(&params.old_string).count();
        if count == 0 {
            return Err(McpError::invalid_params(
                "old_string 在文件中未找到，请先用 docs_read 确认原文（必须逐字符一致，包括空格与换行）",
                None,
            ));
        }
        if count > 1 && !params.replace_all.unwrap_or(false) {
            return Err(McpError::invalid_params(
                format!(
                    "old_string 在文件中出现 {count} 次，请补充上下文使其唯一，或传 replace_all=true 全部替换"
                ),
                None,
            ));
        }

        let updated = if params.replace_all.unwrap_or(false) {
            content.replace(&params.old_string, &params.new_string)
        } else {
            content.replacen(&params.old_string, &params.new_string, 1)
        };
        let entry = docs::save_file(&self.store, &params.root_id, &path, &updated)
            .map_err(Self::into_error)?;
        self.notify_docs_change(&params.root_id, &path, "edit");
        serde_json::to_string_pretty(&entry).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 搜索文档
    #[tool(
        name = "docs_search",
        description = "在指定文档库中按关键词搜索文件（匹配文件名与内容），返回命中文件列表。"
    )]
    async fn docs_search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<String, McpError> {
        let results = docs::search(&self.store, &params.root_id, &params.keyword)
            .map_err(Self::into_error)?;
        serde_json::to_string_pretty(&results).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 删除文件或文件夹
    #[tool(
        name = "docs_delete",
        description = "删除 Eidolon 文档库中的文件或文件夹（谨慎使用，文件夹会连同内容一起删除）。"
    )]
    async fn docs_delete(
        &self,
        Parameters(params): Parameters<PathParams>,
    ) -> Result<String, McpError> {
        let path = Self::relative(&params.path);
        let message = docs::delete_entry(&self.store, &params.root_id, &path)
            .map_err(Self::into_error)?;
        self.notify_docs_change(&params.root_id, &path, "delete");
        Ok(message)
    }

    /// 列出个人任务
    #[tool(
        name = "task_list",
        description = "列出 Eidolon 个人任务清单中的任务（可按状态过滤），按清单排序返回，含 id、标题、备注、状态、优先级、截止日期、完成时间。"
    )]
    async fn task_list(
        &self,
        Parameters(params): Parameters<TaskListParams>,
    ) -> Result<String, McpError> {
        let mut tasks = TaskRepository::new(&self.database)
            .list()
            .map_err(Self::into_error)?;
        if let Some(status) = params
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            match status.to_lowercase().as_str() {
                "all" => {}
                "todo" => tasks.retain(|task| task.status == TASK_STATUS_TODO),
                "doing" => tasks.retain(|task| task.status == TASK_STATUS_DOING),
                "done" => tasks.retain(|task| task.status == TASK_STATUS_DONE),
                other => {
                    return Err(McpError::invalid_params(
                        format!("status 只支持 todo | doing | done | all，收到 {other}"),
                        None,
                    ));
                }
            }
        }
        serde_json::to_string_pretty(&tasks).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 新建任务
    #[tool(
        name = "task_create",
        description = "在 Eidolon 个人任务清单中新建任务（状态默认 todo）。返回新建的任务（含 id）。"
    )]
    async fn task_create(
        &self,
        Parameters(params): Parameters<TaskCreateParams>,
    ) -> Result<String, McpError> {
        let task = Task {
            title: params.title.trim().to_string(),
            notes: params.notes.unwrap_or_default(),
            priority: params
                .priority
                .unwrap_or_else(|| TASK_PRIORITY_NONE.to_string()),
            due_date: params.due_date.unwrap_or_default(),
            ..Task::default()
        };
        let saved = TaskRepository::new(&self.database)
            .upsert(&task)
            .map_err(Self::into_error)?;
        serde_json::to_string_pretty(&saved).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 更新任务（只改传入的字段）
    #[tool(
        name = "task_update",
        description = "更新 Eidolon 个人任务：只传需要修改的字段，未传的保持不变；备注/截止日期传空字符串即清除。返回更新后的任务。"
    )]
    async fn task_update(
        &self,
        Parameters(params): Parameters<TaskUpdateParams>,
    ) -> Result<String, McpError> {
        let repo = TaskRepository::new(&self.database);
        let mut task = Self::find_task(&repo, &params.task_id)?;

        if let Some(title) = params.title {
            task.title = title;
        }
        if let Some(notes) = params.notes {
            task.notes = notes;
        }
        if let Some(priority) = params.priority {
            task.priority = priority;
        }
        if let Some(due_date) = params.due_date {
            task.due_date = due_date;
        }
        if let Some(status) = params.status {
            task.status = status;
        }

        let saved = repo.upsert(&task).map_err(Self::into_error)?;
        serde_json::to_string_pretty(&saved).map_err(|error| Self::into_error(error.to_string()))
    }

    /// 变更任务状态
    #[tool(
        name = "task_set_status",
        description = "变更 Eidolon 个人任务的状态：todo（待办）、doing（进行中）、done（完成，自动记录完成时间；改回未完成会清除）。返回更新后的任务。"
    )]
    async fn task_set_status(
        &self,
        Parameters(params): Parameters<TaskSetStatusParams>,
    ) -> Result<String, McpError> {
        let repo = TaskRepository::new(&self.database);
        let mut task = Self::find_task(&repo, &params.task_id)?;

        let status = normalize_status(&params.status);
        if status != params.status.trim().to_lowercase() {
            return Err(McpError::invalid_params(
                format!("status 只支持 todo | doing | done，收到 {}", params.status),
                None,
            ));
        }
        task.status = status;

        let saved = repo.upsert(&task).map_err(Self::into_error)?;
        serde_json::to_string_pretty(&saved).map_err(|error| Self::into_error(error.to_string()))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EidolonMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "eidolon".to_string(),
                title: Some("Eidolon".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some("通过 MCP 操作 Eidolon 桌面应用的文档库与个人任务".to_string()),
                ..Default::default()
            },
            instructions: Some(
                "文档工具直接读写用户 Eidolon 文档库中的 Markdown 文件（与 Eidolon 界面、Obsidian 共享同一份磁盘文件），\
                 先调用 docs_list_roots 获取文档库 id，再浏览/读写。\
                 局部修改已有文档优先用 docs_edit（只传改动片段），新建或整体重写才用 docs_write。\
                 任务工具操作用户的个人任务清单（task_list / task_create / task_update / task_set_status），\
                 没有删除能力；状态只支持 todo / doing / done。"
                    .to_string(),
            ),
        }
    }
}

// ============================================================
// HTTP 服务：token 鉴权 + streamable-http MCP
// ============================================================

async fn authorize(
    State(expected): State<Arc<String>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let provided = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or("");

    if provided.is_empty() || !constant_time_eq(provided, expected.as_str()) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// 生产环境的文档变更回调：通过 AppHandle 把变更广播给所有窗口
pub fn docs_change_emitter(app: tauri::AppHandle) -> DocsChangeEmitter {
    Arc::new(move |change| {
        use tauri::Emitter;
        if let Err(error) = app.emit(DOCS_EXTERNAL_CHANGE_EVENT, change) {
            log::error!("发送文档外部变更事件失败: {error}");
        }
    })
}

/// 在 127.0.0.1 上启动 MCP 服务，返回实际绑定的端口。
/// `shutdown` 取消后服务停止（活动会话随之关闭）。
pub async fn run_server(
    settings: &McpServerSettings,
    store: Arc<LocalJsonStore>,
    database: Arc<ApiClientDatabase>,
    on_docs_change: DocsChangeEmitter,
    shutdown: CancellationToken,
) -> Result<u16, String> {
    let token = Arc::new(settings.token.trim().to_string());
    if token.is_empty() {
        return Err("MCP 服务访问令牌为空，请先在设置中启用以生成".to_string());
    }

    let session_manager = Arc::new(LocalSessionManager::default());
    let cancellation = shutdown.clone();
    let service = StreamableHttpService::new(
        move || Ok(EidolonMcpServer::new(store.clone(), database.clone(), on_docs_change.clone())),
        session_manager,
        StreamableHttpServerConfig {
            cancellation_token: cancellation,
            ..Default::default()
        },
    );

    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(token, authorize));

    // 配置端口被占用时向后探测，避免与其他本机服务冲突
    let mut last_error = String::new();
    for offset in 0..=PORT_PROBE_LIMIT {
        let port = settings.port.saturating_add(offset);
        match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => {
                // port=0（临时端口）与探测回退都以实际地址为准
                let actual = listener
                    .local_addr()
                    .map_err(|error| format!("读取监听地址失败: {error}"))?
                    .port();
                log::info!("MCP 服务已启动: http://127.0.0.1:{actual}/mcp");
                let shutdown_for_serve = shutdown.clone();
                tokio::spawn(async move {
                    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                        shutdown_for_serve.cancelled().await;
                    });
                    if let Err(error) = server.await {
                        log::error!("MCP 服务异常退出: {error}");
                    }
                });
                return Ok(actual);
            }
            Err(error) => {
                last_error = format!("端口 {port}: {error}");
            }
        }
    }

    Err(format!(
        "无法绑定 MCP 服务端口（尝试 {} 起）: {last_error}",
        settings.port
    ))
}

// ============================================================
// 生命周期管理（由 tauri State 持有）
// ============================================================

/// MCP 服务的运行句柄：启动/停止/查询，设置变更时重启。
/// 服务任务内部随 `CancellationToken` 优雅退出。
#[derive(Default)]
pub struct McpServerHandle {
    shutdown: tokio::sync::Mutex<Option<CancellationToken>>,
    active_port: tokio::sync::Mutex<Option<u16>>,
}

impl McpServerHandle {
    pub async fn is_running(&self) -> bool {
        self.shutdown.lock().await.is_some()
    }

    pub async fn active_port(&self) -> Option<u16> {
        *self.active_port.lock().await
    }

    /// 启动服务。已在运行时先停止旧实例。
    pub async fn start(
        &self,
        settings: &McpServerSettings,
        store: Arc<LocalJsonStore>,
        database: Arc<ApiClientDatabase>,
        on_docs_change: DocsChangeEmitter,
    ) -> Result<u16, String> {
        self.stop().await;

        let shutdown = CancellationToken::new();
        let actual_port = run_server(settings, store, database, on_docs_change, shutdown.clone())
            .await?;

        *self.shutdown.lock().await = Some(shutdown);
        *self.active_port.lock().await = Some(actual_port);
        Ok(actual_port)
    }

    /// 停止服务（未运行时无操作）。取消后 axum 优雅退出并关闭活动会话。
    pub async fn stop(&self) {
        if let Some(shutdown) = self.shutdown.lock().await.take() {
            shutdown.cancel();
        }
        *self.active_port.lock().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mcp_server::McpServerSettings;

    fn temp_store(tag: &str) -> (tempfile::TempDir, Arc<LocalJsonStore>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Arc::new(LocalJsonStore::new(dir.path().join(tag)).expect("store"));
        (dir, store)
    }

    fn test_database() -> Arc<ApiClientDatabase> {
        Arc::new(ApiClientDatabase::open_in_memory().expect("in-memory database"))
    }

    #[test]
    fn token_generated_when_empty() {
        let (_dir, store) = temp_store("settings");
        let settings = load_or_init(&store).expect("init");
        assert!(!settings.token.is_empty());
        assert_eq!(settings.token.len(), 43);

        // 再次读取幂等，不换令牌
        let again = load_or_init(&store).expect("reload");
        assert_eq!(again.token, settings.token);
    }

    #[tokio::test]
    async fn tools_roundtrip_and_sandbox() {
        let (dir, store) = temp_store("tools");
        let root_path = dir.path().join("docs-root");
        std::fs::create_dir_all(&root_path).expect("create root");

        // 注册一个文档库（保存时 id 按路径哈希重生成，从保存结果取真实 id）
        let mut docs_settings = docs::get_settings(&store);
        docs_settings.roots.push(crate::models::docs::DocsRoot {
            id: String::new(),
            path: root_path.to_string_lossy().to_string(),
        });
        let docs_settings =
            docs::save_settings(&store, &docs_settings).expect("save docs settings");
        let root_id = docs_settings.roots[0].id.clone();

        // 捕获文档变更事件，验证写操作会通知前端
        let events: Arc<std::sync::Mutex<Vec<DocsExternalChange>>> = Arc::default();
        let emitter: DocsChangeEmitter = {
            let events = events.clone();
            Arc::new(move |change| events.lock().expect("lock").push(change))
        };
        let server = EidolonMcpServer::new(store.clone(), test_database(), emitter);

        // 列根目录
        let roots = server.docs_list_roots().await.expect("roots");
        assert!(roots.contains(&root_id), "got {roots}");

        // 写入（自动建父目录）→ 读取 → 搜索
        let written = server
            .docs_write(Parameters(WriteParams {
                root_id: root_id.clone(),
                path: "会议/周会.md".to_string(),
                content: "# 周会纪要\n\n内容".to_string(),
            }))
            .await
            .expect("write");
        assert!(written.contains("周会.md"), "got {written}");

        let content = server
            .docs_read(Parameters(PathParams {
                root_id: root_id.clone(),
                path: "会议/周会.md".to_string(),
            }))
            .await
            .expect("read");
        assert_eq!(content, "# 周会纪要\n\n内容");

        let hits = server
            .docs_search(Parameters(SearchParams {
                root_id: root_id.clone(),
                keyword: "周会".to_string(),
            }))
            .await
            .expect("search");
        assert!(hits.contains("周会.md"), "got {hits}");

        // 写入成功 → 广播一条 write 事件
        {
            let events = events.lock().expect("lock");
            assert_eq!(events.len(), 1, "got {events:?}");
            assert_eq!(events[0].root_id, root_id);
            assert_eq!(events[0].path, "会议/周会.md");
            assert_eq!(events[0].action, "write");
        }

        // 沙箱：越界路径被拒绝
        let escape = server
            .docs_read(Parameters(PathParams {
                root_id: root_id.clone(),
                path: "/../outside.md".to_string(),
            }))
            .await;
        assert!(escape.is_err(), "越界读取必须失败");

        // 删除
        server
            .docs_delete(Parameters(PathParams {
                root_id: root_id.clone(),
                path: "会议/周会.md".to_string(),
            }))
            .await
            .expect("delete");
        let entries = server
            .docs_list(Parameters(RootParams {
                root_id: root_id.clone(),
            }))
            .await
            .expect("list");
        assert!(!entries.contains("周会.md"), "got {entries}");

        // 删除成功 → 追加一条 delete 事件
        let events = events.lock().expect("lock");
        assert_eq!(events.len(), 2, "got {events:?}");
        assert_eq!(events[1].action, "delete");
    }

    #[tokio::test]
    async fn docs_edit_patch_semantics() {
        let (dir, store) = temp_store("edit");
        let root_path = dir.path().join("docs-root");
        std::fs::create_dir_all(&root_path).expect("create root");

        let mut docs_settings = docs::get_settings(&store);
        docs_settings.roots.push(crate::models::docs::DocsRoot {
            id: String::new(),
            path: root_path.to_string_lossy().to_string(),
        });
        let docs_settings =
            docs::save_settings(&store, &docs_settings).expect("save docs settings");
        let root_id = docs_settings.roots[0].id.clone();

        // 捕获 edit 事件：只有成功的编辑会广播
        let events: Arc<std::sync::Mutex<Vec<DocsExternalChange>>> = Arc::default();
        let emitter: DocsChangeEmitter = {
            let events = events.clone();
            Arc::new(move |change| events.lock().expect("lock").push(change))
        };
        let server = EidolonMcpServer::new(store, test_database(), emitter);
        let path = "笔记.md";
        server
            .docs_write(Parameters(WriteParams {
                root_id: root_id.clone(),
                path: path.to_string(),
                content: "# 标题\n\n第一段。\n\n第二段提到标题。\n".to_string(),
            }))
            .await
            .expect("seed");

        let edit = |old: &str, new: &str, replace_all: Option<bool>| {
            server.docs_edit(Parameters(EditParams {
                root_id: root_id.clone(),
                path: path.to_string(),
                old_string: old.to_string(),
                new_string: new.to_string(),
                replace_all,
            }))
        };

        // 唯一匹配：只替换一处（"标题"出现两次，但带上下文后唯一）
        edit("第一段。", "第一段改了。", None)
            .await
            .expect("edit");
        let content = server
            .docs_read(Parameters(PathParams {
                root_id: root_id.clone(),
                path: path.to_string(),
            }))
            .await
            .expect("read");
        assert_eq!(content, "# 标题\n\n第一段改了。\n\n第二段提到标题。\n");

        // 多处匹配且未传 replace_all → 报错，文件不动
        let ambiguous = edit("标题", "题目", None).await;
        assert!(ambiguous.is_err(), "ambiguous match must be rejected");
        let unchanged = server
            .docs_read(Parameters(PathParams {
                root_id: root_id.clone(),
                path: path.to_string(),
            }))
            .await
            .expect("read");
        assert_eq!(unchanged, "# 标题\n\n第一段改了。\n\n第二段提到标题。\n");

        // replace_all=true → 全部替换
        edit("标题", "题目", Some(true)).await.expect("replace all");
        let replaced = server
            .docs_read(Parameters(PathParams {
                root_id: root_id.clone(),
                path: path.to_string(),
            }))
            .await
            .expect("read");
        assert_eq!(replaced, "# 题目\n\n第一段改了。\n\n第二段提到题目。\n");

        // 未找到 → 报参数错误
        let missing = edit("不存在的原文", "x", None).await;
        assert!(missing.is_err(), "missing old_string must be rejected");

        // 空 old_string → 报参数错误
        let empty = edit("", "x", None).await;
        assert!(empty.is_err(), "empty old_string must be rejected");

        // 文件不存在 → 报错（编辑不负责创建）
        let no_file = server
            .docs_edit(Parameters(EditParams {
                root_id: root_id.clone(),
                path: "不存在.md".to_string(),
                old_string: "a".to_string(),
                new_string: "b".to_string(),
                replace_all: None,
            }))
            .await;
        assert!(no_file.is_err(), "editing missing file must fail");

        // 种子 write + 两次成功编辑（唯一替换 + replace_all）各广播一条，失败的不广播
        let events = events.lock().expect("lock");
        assert_eq!(events.len(), 3, "got {events:?}");
        assert_eq!(events[0].action, "write");
        assert!(events[1..].iter().all(|change| change.action == "edit"));
    }

    #[tokio::test]
    async fn task_tools_roundtrip_without_delete() {
        let (_dir, store) = temp_store("tasks");
        let server = EidolonMcpServer::new(store, test_database(), Arc::new(|_| {}));

        // 新建：默认 todo / none
        let created = server
            .task_create(Parameters(TaskCreateParams {
                title: "写周报".to_string(),
                notes: Some("先拉 git 提交记录".to_string()),
                priority: Some("high".to_string()),
                due_date: Some("2026-10-15".to_string()),
            }))
            .await
            .expect("create");
        assert!(created.contains("\"todo\""), "got {created}");
        assert!(created.contains("task_"), "got {created}");

        let id = serde_json::from_str::<Task>(&created)
            .expect("created json")
            .id;

        // 列表 + 状态过滤
        let all = server
            .task_list(Parameters(TaskListParams { status: None }))
            .await
            .expect("list");
        assert!(all.contains("写周报"), "got {all}");
        let doing = server
            .task_list(Parameters(TaskListParams {
                status: Some("doing".to_string()),
            }))
            .await
            .expect("list doing");
        assert!(
            !doing.contains("写周报"),
            "doing filter should exclude, got {doing}"
        );
        let bad_filter = server
            .task_list(Parameters(TaskListParams {
                status: Some("urgent".to_string()),
            }))
            .await;
        assert!(
            bad_filter.is_err(),
            "unknown status filter must be rejected"
        );

        // 部分更新：只改标题，其余保持
        let updated = server
            .task_update(Parameters(TaskUpdateParams {
                task_id: id.clone(),
                title: Some("整理周报数据".to_string()),
                notes: None,
                priority: None,
                due_date: None,
                status: None,
            }))
            .await
            .expect("update");
        let updated_task = serde_json::from_str::<Task>(&updated).expect("updated json");
        assert_eq!(updated_task.title, "整理周报数据");
        assert_eq!(updated_task.notes, "先拉 git 提交记录");
        assert_eq!(updated_task.priority, "high");
        assert_eq!(updated_task.due_date, "2026-10-15");

        // 状态变更：置 done 补记完成时间，改回 todo 清除
        let done = server
            .task_set_status(Parameters(TaskSetStatusParams {
                task_id: id.clone(),
                status: "done".to_string(),
            }))
            .await
            .expect("done");
        assert!(serde_json::from_str::<Task>(&done)
            .expect("done json")
            .completed_at
            .is_some());

        let reopened = server
            .task_set_status(Parameters(TaskSetStatusParams {
                task_id: id.clone(),
                status: "todo".to_string(),
            }))
            .await
            .expect("reopen");
        assert_eq!(
            serde_json::from_str::<Task>(&reopened)
                .expect("reopened json")
                .completed_at,
            None
        );

        // 非法状态被拒绝
        let bad_status = server
            .task_set_status(Parameters(TaskSetStatusParams {
                task_id: id.clone(),
                status: "archived".to_string(),
            }))
            .await;
        assert!(bad_status.is_err(), "unknown status must be rejected");

        // 不存在的任务报参数错误
        let missing = server
            .task_update(Parameters(TaskUpdateParams {
                task_id: "task_missing".to_string(),
                title: Some("x".to_string()),
                notes: None,
                priority: None,
                due_date: None,
                status: None,
            }))
            .await;
        assert!(missing.is_err(), "unknown task must be rejected");
    }

    #[tokio::test]
    async fn http_server_requires_token() {
        let (_dir, store) = temp_store("http");
        // port=0：由系统分配临时端口，run_server 返回实际端口
        let settings = McpServerSettings {
            enabled: true,
            port: 0,
            token: "test-token-123".to_string(),
        };

        let shutdown = CancellationToken::new();
        let port = run_server(
            &settings,
            store,
            test_database(),
            Arc::new(|_| {}),
            shutdown.clone(),
        )
        .await
        .expect("start");
        assert!(port > 0);

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/mcp");

        // 无令牌 → 401
        let denied = client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("req");
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

        // 带令牌 → 进入 MCP 层（协议报错也行，反正不是 401）
        let accepted = client
            .post(&url)
            .header("authorization", "Bearer test-token-123")
            .json(
                &serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            } }),
            )
            .send()
            .await
            .expect("req");
        assert_ne!(accepted.status(), StatusCode::UNAUTHORIZED);

        shutdown.cancel();
    }
}

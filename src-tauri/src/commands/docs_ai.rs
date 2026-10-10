use tauri::State;

use crate::db::local_store::LocalJsonStore;
use crate::models::docs::{DocsAiChatMessage, DocsAiChatResult};
use crate::services::docs_ai;

/// 文档 AI 助手对话。会话历史由前端持有（内存态，不落库），
/// 后端只负责解析默认模型并完成一轮调用。
#[tauri::command]
pub async fn chat_docs_ai(
    store: State<'_, LocalJsonStore>,
    history: Vec<DocsAiChatMessage>,
    document: String,
    message: String,
) -> Result<DocsAiChatResult, String> {
    docs_ai::run_docs_ai_turn(&store, &history, &document, &message).await
}

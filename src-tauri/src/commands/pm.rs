use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::pm_conversation::PmConversationRepository;
use crate::models::pm::{PmConversation, PmMessage, PmSettings, PmSkillSummary};
use crate::services::pm_chat;
use crate::services::pm_skills;

const PM_SETTINGS_FILENAME: &str = "pm_settings";

fn builtin_skills_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("定位应用资源目录失败: {error}"))?;
    Ok(resource_dir.join("pm-skills"))
}

fn read_pm_settings(store: &LocalJsonStore) -> PmSettings {
    store.read(PM_SETTINGS_FILENAME).unwrap_or_default()
}

fn normalize_pm_settings(settings: &PmSettings) -> Result<PmSettings, String> {
    let override_dir = settings
        .skills_override_dir
        .as_deref()
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
        .map(|dir| {
            let path = PathBuf::from(dir);
            if !path.is_absolute() {
                return Err("覆盖目录必须是绝对路径".to_string());
            }
            Ok(dir.to_string())
        })
        .transpose()?;

    Ok(PmSettings {
        skills_override_dir: override_dir,
    })
}

#[tauri::command]
pub fn list_pm_conversations(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<PmConversation>, String> {
    let repo = PmConversationRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn create_pm_conversation(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<PmConversation, String> {
    let repo = PmConversationRepository::new(&store);
    repo.create()
}

#[tauri::command]
pub fn rename_pm_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
    title: String,
) -> Result<PmConversation, String> {
    let repo = PmConversationRepository::new(&store);
    repo.rename(&conversation_id, &title)
}

#[tauri::command]
pub fn delete_pm_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
) -> Result<String, String> {
    let repo = PmConversationRepository::new(&store);
    repo.delete(&conversation_id)
}

#[tauri::command]
pub fn list_pm_conversation_messages(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
) -> Result<Vec<PmMessage>, String> {
    let repo = PmConversationRepository::new(&store);
    repo.list_messages(&conversation_id)
}

/// 发送一轮对话。先完整跑通路由与生成，成功后才把用户与助手消息一并落库；
/// 失败时返回 Err，不写入任何内容（前端 toast 报错并保留输入）。
#[tauri::command]
pub async fn send_pm_message(
    app: AppHandle,
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
    content: String,
) -> Result<PmMessage, String> {
    let repo = PmConversationRepository::new(&store);
    repo.get(&conversation_id)?
        .ok_or_else(|| "未找到会话，可能已被删除".to_string())?;

    let history = repo.list_messages(&conversation_id)?;
    let settings = read_pm_settings(&store);
    let builtin_dir = builtin_skills_dir(&app)?;

    let result = pm_chat::run_pm_turn(
        &store,
        &builtin_dir,
        settings.skills_override_dir.as_deref(),
        &history,
        &content,
    )
    .await?;

    repo.append_user_message(&conversation_id, content.trim())?;
    repo.append_assistant_message(&conversation_id, &result.content, result.skills_used)
}

#[tauri::command]
pub fn list_pm_skills(
    app: AppHandle,
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<PmSkillSummary>, String> {
    let settings = read_pm_settings(&store);
    let skills = pm_skills::load_skill_library(
        &builtin_skills_dir(&app)?,
        settings.skills_override_dir.as_deref(),
    );

    Ok(skills
        .into_iter()
        .map(|skill| PmSkillSummary {
            slug: skill.slug,
            name: skill.name,
            description: skill.description,
            source: skill.source,
            size: skill.size,
        })
        .collect())
}

#[tauri::command]
pub fn get_pm_skill_content(
    app: AppHandle,
    store: tauri::State<'_, LocalJsonStore>,
    slug: String,
) -> Result<String, String> {
    let settings = read_pm_settings(&store);
    let skills = pm_skills::load_skill_library(
        &builtin_skills_dir(&app)?,
        settings.skills_override_dir.as_deref(),
    );

    skills
        .into_iter()
        .find(|skill| skill.slug == slug)
        .map(|skill| skill.content)
        .ok_or_else(|| format!("未找到框架: {slug}"))
}

#[tauri::command]
pub fn get_pm_settings(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<PmSettings, String> {
    Ok(read_pm_settings(&store))
}

#[tauri::command]
pub fn upsert_pm_settings(
    store: tauri::State<'_, LocalJsonStore>,
    settings: PmSettings,
) -> Result<PmSettings, String> {
    let normalized = normalize_pm_settings(&settings)?;
    store.write(PM_SETTINGS_FILENAME, &normalized)?;
    Ok(normalized)
}

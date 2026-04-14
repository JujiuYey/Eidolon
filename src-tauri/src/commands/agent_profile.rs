use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::agent_profile::AgentProfileRepository;
use crate::models::agent_profile::AgentProfile;

#[tauri::command]
pub fn list_agent_profiles(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<AgentProfile>, String> {
    let repo = AgentProfileRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn get_agent_profile(
    store: tauri::State<'_, LocalJsonStore>,
    profile_id: String,
) -> Result<Option<AgentProfile>, String> {
    let repo = AgentProfileRepository::new(&store);
    repo.get(&profile_id)
}

#[tauri::command]
pub fn upsert_agent_profile(
    store: tauri::State<'_, LocalJsonStore>,
    profile: AgentProfile,
) -> Result<String, String> {
    let repo = AgentProfileRepository::new(&store);
    repo.upsert(&profile)
}

#[tauri::command]
pub fn delete_agent_profile(
    store: tauri::State<'_, LocalJsonStore>,
    profile_id: String,
) -> Result<String, String> {
    let repo = AgentProfileRepository::new(&store);
    repo.delete(&profile_id)
}

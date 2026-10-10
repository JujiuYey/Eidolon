use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::git_account_repo::GitAccountRepository;
use crate::models::git::GitAccount;

// ===== Git 账户 =====

#[tauri::command]
pub fn list_git_accounts(
    database: tauri::State<'_, ApiClientDatabase>,
) -> Result<Vec<GitAccount>, String> {
    GitAccountRepository::new(&database).list()
}

#[tauri::command]
pub fn upsert_git_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account: GitAccount,
) -> Result<GitAccount, String> {
    GitAccountRepository::new(&database).upsert(&account)
}

#[tauri::command]
pub fn delete_git_account(
    database: tauri::State<'_, ApiClientDatabase>,
    account_id: String,
) -> Result<String, String> {
    GitAccountRepository::new(&database).delete(&account_id)
}

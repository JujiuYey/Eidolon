//! 代码导入命令：执行框架导出、写入导入结果、维护导入源。

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::api_import_repo::ApiImportRepository;
use crate::models::api_client::{
    ApiImportSource, ImportGroupSpec, ImportRequestSpec, ImportSummary,
};
use crate::services::code_export;

/// 在后端仓库执行框架导出命令，返回清单 JSON 文本。
/// 前端适配器负责把它解析为分组/请求定义。
#[tauri::command]
pub async fn export_code_manifest(
    framework: String,
    repo_path: String,
    export_id: String,
) -> Result<String, String> {
    code_export::export(&framework, &repo_path, &export_id).await
}

/// 取消正在进行的导出任务
#[tauri::command]
pub async fn cancel_code_export(export_id: String) -> Result<bool, String> {
    code_export::cancel(&export_id).await
}

/// 查询项目绑定的导入源（每项目至多一条），向导用于回填
#[tauri::command]
pub fn list_api_import_sources(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
) -> Result<Vec<ApiImportSource>, String> {
    ApiImportRepository::new(&database).list_sources(&project_id)
}

/// 执行导入：保存导入源并按清单重建分组、批量写入请求。
/// `replace_existing` 为真时先清空项目现有分组（含其请求与历史）。
#[tauri::command]
pub fn run_api_import(
    database: tauri::State<'_, ApiClientDatabase>,
    project_id: String,
    framework: String,
    repo_path: String,
    replace_existing: bool,
    groups: Vec<ImportGroupSpec>,
    requests: Vec<ImportRequestSpec>,
) -> Result<ImportSummary, String> {
    let repository = ApiImportRepository::new(&database);
    let summary = repository.run_import(&project_id, replace_existing, &groups, &requests)?;
    repository.save_source(&project_id, &framework, &repo_path)?;
    Ok(summary)
}

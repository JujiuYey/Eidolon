use std::path::Path;

use crate::services::codegen::{
    go_generator, go_parser, sql_parser, ts_generator,
    types::{GeneratedGoCrudResult, GoCodeGenConfig, ParsedTable},
};

#[tauri::command]
pub async fn generate_crud(
    resource_path: String,
    frontend_base_path: String,
    overwrite: Option<bool>,
) -> Result<Vec<String>, String> {
    if !Path::new(&resource_path).exists() {
        return Err(format!("Resource 文件不存在: {resource_path}"));
    }

    if !Path::new(&frontend_base_path).is_dir() {
        return Err(format!("前端目录不存在: {frontend_base_path}"));
    }

    let entity = go_parser::parse_go_resource(&resource_path)?;
    ts_generator::generate_crud_files(&entity, &frontend_base_path, overwrite.unwrap_or(false))
}

#[tauri::command]
pub async fn parse_sql_ddl(sql: String) -> Result<ParsedTable, String> {
    sql_parser::parse_sql_ddl(&sql)
}

#[tauri::command]
pub async fn generate_go_crud(config: GoCodeGenConfig) -> Result<GeneratedGoCrudResult, String> {
    go_generator::generate_go_crud_files(&config)
}

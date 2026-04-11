use std::fs;
use std::path::{Path, PathBuf};

use super::types::{AuditType, GeneratedGoCrudResult, GoCodeGenConfig, ParsedField};

pub fn generate_go_crud_files(config: &GoCodeGenConfig) -> Result<GeneratedGoCrudResult, String> {
    validate_config(config)?;

    let base_path = Path::new(&config.output_dir);
    fs::create_dir_all(base_path)
        .map_err(|error| format!("创建输出目录失败 {}: {error}", base_path.display()))?;

    let entity_file_name = to_snake_case(&config.entity_name);
    let specs = vec![
        (
            format!("model/{entity_file_name}.go"),
            generate_model_content(config)?,
        ),
        (
            format!("payload/{entity_file_name}.go"),
            generate_payload_content(config)?,
        ),
        (
            format!("resource/{entity_file_name}.go"),
            generate_resource_content(config)?,
        ),
    ];

    let existing_files = specs
        .iter()
        .map(|(relative_path, _)| (relative_path, base_path.join(relative_path)))
        .filter(|(_, path)| path.exists())
        .map(|(relative_path, _)| relative_path.to_string())
        .collect::<Vec<_>>();

    if !config.overwrite && !existing_files.is_empty() {
        return Err(format!(
            "以下文件已存在，请开启覆盖后重试:\n{}",
            existing_files.join("\n")
        ));
    }

    let mut generated_files = Vec::with_capacity(specs.len());
    for (relative_path, content) in specs {
        let absolute_path = base_path.join(&relative_path);
        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建目录失败 {}: {error}", parent.display()))?;
        }

        fs::write(&absolute_path, content)
            .map_err(|error| format!("写入文件失败 {}: {error}", absolute_path.display()))?;
        generated_files.push(path_to_string(absolute_path));
    }

    Ok(GeneratedGoCrudResult {
        generated_files,
        module_registration_snippet: format!(
            "vef.ProvideAPIResource(resource.New{}Resource),",
            config.entity_name
        ),
    })
}

fn validate_config(config: &GoCodeGenConfig) -> Result<(), String> {
    if config.entity_name.trim().is_empty() {
        return Err("实体名称不能为空".to_string());
    }
    if config.module_path.trim().is_empty() {
        return Err("模块路径不能为空".to_string());
    }
    if config.table_name.trim().is_empty() {
        return Err("表名不能为空".to_string());
    }
    if config.table_alias.trim().is_empty() {
        return Err("表别名不能为空".to_string());
    }
    if config.rpc_path.trim().is_empty() {
        return Err("RPC 路径不能为空".to_string());
    }
    if config.go_module_prefix.trim().is_empty() {
        return Err("Go 模块前缀不能为空".to_string());
    }
    if config.output_dir.trim().is_empty() {
        return Err("输出目录不能为空".to_string());
    }
    if config.fields.is_empty() {
        return Err("至少需要一个字段".to_string());
    }
    if ![
        config.enable_find_page,
        config.enable_create,
        config.enable_update,
        config.enable_delete,
        config.enable_delete_many,
    ]
    .into_iter()
    .any(|enabled| enabled)
    {
        return Err("至少需要启用一个 CRUD 操作".to_string());
    }

    Ok(())
}

fn generate_model_content(config: &GoCodeGenConfig) -> Result<String, String> {
    let mut imports = vec!["\"github.com/coldsmirk/vef-framework-go/orm\"".to_string()];
    if uses_time_type(&config.fields) {
        imports.push("\"time\"".to_string());
    }

    let mut lines = Vec::new();
    lines.push("package model".to_string());
    lines.push(String::new());
    lines.push(render_import_block(&imports));
    lines.push(String::new());
    lines.push(format!("// {} 自动生成模型.", config.entity_name));
    lines.push(format!("type {} struct {{", config.entity_name));
    lines.push(format!(
        "\torm.BaseModel `bun:\"table:{},alias:{}\"`",
        config.table_name, config.table_alias
    ));
    if let Some(audit_model_type) = audit_model_type(config.audit_type) {
        lines.push(format!("\torm.{audit_model_type}"));
    }
    lines.push(String::new());
    lines.extend(render_model_fields(&config.fields));
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

fn generate_payload_content(config: &GoCodeGenConfig) -> Result<String, String> {
    let mut imports = vec!["\"github.com/coldsmirk/vef-framework-go/api\"".to_string()];
    if uses_time_type(&config.fields) {
        imports.push("\"time\"".to_string());
    }

    let search_column = preferred_search_column(&config.fields);
    let mut lines = Vec::new();
    lines.push("package payload".to_string());
    lines.push(String::new());
    lines.push(render_import_block(&imports));
    lines.push(String::new());
    lines.push(format!("// {}Search 自动生成搜索参数.", config.entity_name));
    lines.push(format!("type {}Search struct {{", config.entity_name));
    lines.push("\tapi.P".to_string());
    lines.push(String::new());
    lines.push(format!(
        "\tKeyword string `json:\"keyword\" search:\"contains,column={search_column}\"`"
    ));
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push(format!(
        "// {}Params 自动生成新增/修改参数.",
        config.entity_name
    ));
    lines.push(format!("type {}Params struct {{", config.entity_name));
    lines.push("\tapi.P".to_string());
    lines.push(String::new());
    lines.push("\tID string `json:\"id\"`".to_string());
    lines.extend(render_payload_fields(&config.fields));
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

fn generate_resource_content(config: &GoCodeGenConfig) -> Result<String, String> {
    let module_import_path = format!(
        "{}/{}",
        trim_slashes(&config.go_module_prefix),
        trim_slashes(&config.module_path)
    );

    let mut imports = vec![
        "\"github.com/coldsmirk/vef-framework-go/api\"".to_string(),
        "\"github.com/coldsmirk/vef-framework-go/crud\"".to_string(),
        format!("\"{module_import_path}/model\""),
        format!("\"{module_import_path}/payload\""),
    ];
    if config.enable_sort {
        imports.push("\"github.com/coldsmirk/vef-framework-go/sortx\"".to_string());
        imports.push(format!("\"{module_import_path}/schema\""));
    }

    let mut lines = Vec::new();
    lines.push("package resource".to_string());
    lines.push(String::new());
    lines.push(render_import_block(&imports));
    lines.push(String::new());
    lines.push(format!("// {}Resource 自动生成资源.", config.entity_name));
    lines.push(format!("type {}Resource struct {{", config.entity_name));
    lines.push("\tapi.Resource".to_string());
    if config.enable_find_page {
        lines.push(format!(
            "\tcrud.FindPage[model.{0}, payload.{0}Search]",
            config.entity_name
        ));
    }
    if config.enable_create {
        lines.push(format!(
            "\tcrud.Create[model.{0}, payload.{0}Params]",
            config.entity_name
        ));
    }
    if config.enable_update {
        lines.push(format!(
            "\tcrud.Update[model.{0}, payload.{0}Params]",
            config.entity_name
        ));
    }
    if config.enable_delete {
        lines.push(format!("\tcrud.Delete[model.{}]", config.entity_name));
    }
    if config.enable_delete_many {
        lines.push(format!("\tcrud.DeleteMany[model.{}]", config.entity_name));
    }
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push(format!(
        "func New{}Resource() api.Resource {{",
        config.entity_name
    ));
    lines.push(format!("\treturn &{}Resource{{", config.entity_name));
    lines.push(format!(
        "\t\tResource: api.NewRPCResource(\"{}\"),",
        config.rpc_path
    ));
    if config.enable_find_page {
        lines.push(render_find_page_initializer(config));
    }
    if config.enable_create {
        lines.push(render_simple_initializer(
            "Create",
            &format!(
                "crud.NewCreate[model.{0}, payload.{0}Params]()",
                config.entity_name
            ),
            audit_enabled(config.audit_type),
        ));
    }
    if config.enable_update {
        lines.push(render_simple_initializer(
            "Update",
            &format!(
                "crud.NewUpdate[model.{0}, payload.{0}Params]()",
                config.entity_name
            ),
            audit_enabled(config.audit_type),
        ));
    }
    if config.enable_delete {
        lines.push(render_simple_initializer(
            "Delete",
            &format!("crud.NewDelete[model.{}]()", config.entity_name),
            audit_enabled(config.audit_type),
        ));
    }
    if config.enable_delete_many {
        lines.push(render_simple_initializer(
            "DeleteMany",
            &format!("crud.NewDeleteMany[model.{}]()", config.entity_name),
            audit_enabled(config.audit_type),
        ));
    }
    lines.push("\t}".to_string());
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

fn render_import_block(imports: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push("import (".to_string());
    let mut sorted_imports = imports.to_vec();
    sorted_imports.sort();
    for import in sorted_imports {
        lines.push(format!("\t{import}"));
    }
    lines.push(")".to_string());
    lines.join("\n")
}

fn render_model_fields(fields: &[ParsedField]) -> Vec<String> {
    fields
        .iter()
        .flat_map(|field| {
            let mut lines = Vec::new();
            if let Some(comment) = field.comment.as_deref() {
                lines.push(format!("\t// {comment}"));
            }
            let mut tags = vec![format!("json:\"{}\"", field.json_name)];
            if let Some(validate) = field.validate.as_deref() {
                tags.push(format!("validate:\"{validate}\""));
            }
            if let Some(comment) = field.comment.as_deref() {
                tags.push(format!("label:\"{comment}\""));
            }
            if field.go_type == "map[string]any" {
                tags.push("bun:\"type:jsonb,nullzero\"".to_string());
            }
            let mut line = format!("\t{} {} `{}`", field.go_name, field.go_type, tags.join(" "));
            if let Some(comment) = field.comment.as_deref() {
                line.push_str(&format!(" // {comment}"));
            }
            lines.push(line);
            lines
        })
        .collect()
}

fn render_payload_fields(fields: &[ParsedField]) -> Vec<String> {
    fields
        .iter()
        .flat_map(|field| {
            let mut tags = vec![format!("json:\"{}\"", field.json_name)];
            if let Some(validate) = field.validate.as_deref() {
                tags.push(format!("validate:\"{validate}\""));
            }
            if let Some(comment) = field.comment.as_deref() {
                tags.push(format!("label:\"{comment}\""));
            }
            let mut lines = Vec::new();
            if let Some(comment) = field.comment.as_deref() {
                lines.push(format!("\t// {comment}"));
            }
            let mut line = format!("\t{} {} `{}`", field.go_name, field.go_type, tags.join(" "));
            if let Some(comment) = field.comment.as_deref() {
                line.push_str(&format!(" // {comment}"));
            }
            lines.push(line);
            lines
        })
        .collect()
}

fn render_find_page_initializer(config: &GoCodeGenConfig) -> String {
    let mut chain = vec![format!(
        "crud.NewFindPage[model.{0}, payload.{0}Search]()",
        config.entity_name
    )];
    if config.enable_sort {
        chain.push(format!(
            "WithDefaultSort(&sortx.OrderSpec{{\n\t\t\t\tColumn: schema.{0}.SortOrder(),\n\t\t\t}})",
            config.entity_name
        ));
    }
    if config.enable_audit_user_names {
        chain.push("WithAuditUserNames(model.UserModel)".to_string());
    }

    let mut rendered = format!("\t\tFindPage: {}", chain[0]);
    for operation in chain.iter().skip(1) {
        rendered.push_str(".\n");
        rendered.push_str(&indent_block(operation, "\t\t\t"));
    }
    rendered.push(',');
    rendered
}

fn render_simple_initializer(field_name: &str, initializer: &str, enable_audit: bool) -> String {
    if enable_audit {
        return format!("\t\t{field_name}: {initializer}().\n\t\t\tEnableAudit(),");
    }

    format!("\t\t{field_name}: {initializer},")
}

fn indent_block(block: &str, prefix: &str) -> String {
    block
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn preferred_search_column(fields: &[ParsedField]) -> String {
    fields
        .iter()
        .find(|field| field.name == "name")
        .or_else(|| {
            fields
                .iter()
                .find(|field| matches!(field.go_type.as_str(), "string" | "*string"))
        })
        .map(|field| field.name.clone())
        .unwrap_or_else(|| fields[0].name.clone())
}

fn uses_time_type(fields: &[ParsedField]) -> bool {
    fields
        .iter()
        .any(|field| matches!(field.go_type.as_str(), "time.Time" | "*time.Time"))
}

fn audit_model_type(audit_type: AuditType) -> Option<&'static str> {
    match audit_type {
        AuditType::FullAudited => Some("FullAuditedModel"),
        AuditType::FullTracked => Some("FullTrackedModel"),
        AuditType::CreationAudited => Some("CreationAuditedModel"),
        AuditType::CreationTracked => Some("CreationTrackedModel"),
        AuditType::None => None,
    }
}

fn audit_enabled(audit_type: AuditType) -> bool {
    !matches!(audit_type, AuditType::None)
}

fn trim_slashes(value: &str) -> String {
    value.trim_matches('/').replace('\\', "/")
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn to_snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for (index, ch) in value.chars().enumerate() {
        if ch.is_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.extend(ch.to_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::generate_go_crud_files;
    use crate::services::codegen::types::{
        AuditType, GeneratedGoCrudResult, GoCodeGenConfig, ParsedField,
    };

    fn sample_config(output_dir: String) -> GoCodeGenConfig {
        GoCodeGenConfig {
            entity_name: "User".to_string(),
            module_path: "sys".to_string(),
            table_name: "sys_user".to_string(),
            table_alias: "u".to_string(),
            rpc_path: "smp/sys/user".to_string(),
            go_module_prefix: "smp-server/internal".to_string(),
            output_dir,
            fields: vec![
                ParsedField {
                    name: "organization_id".to_string(),
                    go_name: "OrganizationID".to_string(),
                    json_name: "organizationId".to_string(),
                    go_type: "string".to_string(),
                    db_type: "VARCHAR(32)".to_string(),
                    nullable: false,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("机构主键".to_string()),
                    validate: Some("required,alphanum,max=32".to_string()),
                },
                ParsedField {
                    name: "name".to_string(),
                    go_name: "Name".to_string(),
                    json_name: "name".to_string(),
                    go_type: "string".to_string(),
                    db_type: "VARCHAR(64)".to_string(),
                    nullable: false,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("用户名称".to_string()),
                    validate: Some("required,max=64".to_string()),
                },
                ParsedField {
                    name: "email".to_string(),
                    go_name: "Email".to_string(),
                    json_name: "email".to_string(),
                    go_type: "*string".to_string(),
                    db_type: "VARCHAR(128)".to_string(),
                    nullable: true,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("邮箱地址".to_string()),
                    validate: Some("omitempty,max=128".to_string()),
                },
                ParsedField {
                    name: "is_active".to_string(),
                    go_name: "IsActive".to_string(),
                    json_name: "isActive".to_string(),
                    go_type: "bool".to_string(),
                    db_type: "TINYINT(1)".to_string(),
                    nullable: false,
                    is_primary_key: false,
                    default_value: Some("1".to_string()),
                    comment: Some("是否启用".to_string()),
                    validate: None,
                },
                ParsedField {
                    name: "sort_order".to_string(),
                    go_name: "SortOrder".to_string(),
                    json_name: "sortOrder".to_string(),
                    go_type: "int".to_string(),
                    db_type: "INT".to_string(),
                    nullable: false,
                    is_primary_key: false,
                    default_value: None,
                    comment: Some("排序".to_string()),
                    validate: Some("required,min=0".to_string()),
                },
            ],
            audit_type: AuditType::FullAudited,
            enable_find_page: true,
            enable_create: true,
            enable_update: true,
            enable_delete: true,
            enable_delete_many: false,
            enable_sort: true,
            enable_audit_user_names: true,
            overwrite: false,
        }
    }

    #[test]
    fn test_generate_go_crud_files_writes_expected_files_and_snippet() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("sco-go-crud-generator-{unique}"));
        fs::create_dir_all(&output_dir).expect("create output dir");

        let result =
            generate_go_crud_files(&sample_config(output_dir.to_string_lossy().into_owned()))
                .expect("generator should succeed");

        assert_eq!(
            result,
            GeneratedGoCrudResult {
                generated_files: vec![
                    output_dir
                        .join("model/user.go")
                        .to_string_lossy()
                        .into_owned(),
                    output_dir
                        .join("payload/user.go")
                        .to_string_lossy()
                        .into_owned(),
                    output_dir
                        .join("resource/user.go")
                        .to_string_lossy()
                        .into_owned(),
                ],
                module_registration_snippet: "vef.ProvideAPIResource(resource.NewUserResource),"
                    .to_string(),
            }
        );

        let model = fs::read_to_string(output_dir.join("model/user.go")).expect("read model");
        assert!(model.contains("type User struct"));
        assert!(model.contains("orm.FullAuditedModel"));
        assert!(model.contains("OrganizationID string `json:\"organizationId\" validate:\"required,alphanum,max=32\" label:\"机构主键\"`"));

        let payload = fs::read_to_string(output_dir.join("payload/user.go")).expect("read payload");
        assert!(payload.contains("type UserSearch struct"));
        assert!(
            payload.contains("Keyword string `json:\"keyword\" search:\"contains,column=name\"`")
        );
        assert!(payload.contains("ID string `json:\"id\"`"));

        let resource =
            fs::read_to_string(output_dir.join("resource/user.go")).expect("read resource");
        assert!(resource.contains("type UserResource struct"));
        assert!(resource.contains("crud.FindPage[model.User, payload.UserSearch]"));
        assert!(resource.contains("WithDefaultSort(&sortx.OrderSpec"));
        assert!(resource.contains("WithAuditUserNames(model.UserModel)"));
        assert!(resource.contains("Resource: api.NewRPCResource(\"smp/sys/user\")"));
    }

    #[test]
    fn test_generate_go_crud_files_rejects_existing_files_without_overwrite() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("sco-go-crud-overwrite-{unique}"));
        fs::create_dir_all(output_dir.join("model")).expect("create model dir");
        fs::write(output_dir.join("model/user.go"), "existing").expect("write existing model");

        let error =
            generate_go_crud_files(&sample_config(output_dir.to_string_lossy().into_owned()))
                .expect_err("generator should reject overwrite");

        assert!(error.contains("model/user.go"));
    }
}

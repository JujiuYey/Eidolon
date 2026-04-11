use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::types::{ModelField, ParsedEntity};

const API_TEMPLATE: &str = include_str!("../../../templates/api.ts.tera");
const ROUTE_TEMPLATE: &str = include_str!("../../../templates/route.tsx.tera");
const FORM_TEMPLATE: &str = include_str!("../../../templates/form.tsx.tera");
const BASIC_SEARCH_TEMPLATE: &str = include_str!("../../../templates/basic_search.tsx.tera");
const HELPERS_TEMPLATE: &str = include_str!("../../../templates/helpers.ts.tera");
const AUDIT_FIELD_NAMES: &[&str] = &[
    "id",
    "createdAt",
    "updatedAt",
    "createdBy",
    "updatedBy",
    "createdByName",
    "updatedByName",
];

#[derive(Debug, Clone, Serialize)]
struct InterfaceFieldTemplate {
    json_name: String,
    ts_type: String,
    is_optional: bool,
}

#[derive(Debug, Clone, Serialize)]
struct FormFieldTemplate {
    json_name: String,
    label: String,
    component_kind: String,
    span: usize,
    required: bool,
    validator_mode: Option<String>,
    validator_expr: Option<String>,
    true_label: String,
    false_label: String,
}

#[derive(Debug, Clone, Serialize)]
struct RouteFieldTemplate {
    json_name: String,
    label: String,
    render_kind: String,
    width: usize,
    align: Option<String>,
    should_cell_update: bool,
    true_label: String,
    false_label: String,
}

#[derive(Debug, Clone, Serialize)]
struct SearchFieldTemplate {
    json_name: String,
    label: String,
    placeholder: String,
}

pub fn create_tera_engine() -> Result<tera::Tera, String> {
    let mut tera = tera::Tera::default();
    tera.add_raw_templates(vec![
        ("api.ts", API_TEMPLATE),
        ("route.tsx", ROUTE_TEMPLATE),
        ("form.tsx", FORM_TEMPLATE),
        ("basic_search.tsx", BASIC_SEARCH_TEMPLATE),
        ("helpers.ts", HELPERS_TEMPLATE),
    ])
    .map_err(|error| format!("模板加载失败: {error}"))?;

    Ok(tera)
}

fn default_value_for_field(field: &ModelField) -> &'static str {
    match field.go_type.as_str() {
        "bool" | "*bool" if field.json_name == "isActive" => "true",
        "bool" | "*bool" => "false",
        "[]string" | "[]int" | "[]int32" | "[]int64" | "[]uint" | "[]uint32" | "[]uint64"
        | "[]bool" => "[]",
        "map[string]any" => "{}",
        _ => "undefined",
    }
}

fn to_kebab_case(value: &str) -> String {
    value.replace('_', "-")
}

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => {
            let mut output = first.to_lowercase().to_string();
            output.push_str(chars.as_str());
            output
        }
        None => String::new(),
    }
}

fn is_audit_field(json_name: &str) -> bool {
    AUDIT_FIELD_NAMES.contains(&json_name)
}

fn is_bool_field(field: &ModelField) -> bool {
    matches!(field.go_type.as_str(), "bool" | "*bool")
}

fn is_number_field(field: &ModelField) -> bool {
    matches!(
        field.go_type.as_str(),
        "int"
            | "int32"
            | "int64"
            | "uint"
            | "uint32"
            | "uint64"
            | "*int"
            | "*int32"
            | "*int64"
            | "*uint"
            | "*uint32"
            | "*uint64"
    )
}

fn frontend_entity_name(entity: &ParsedEntity) -> String {
    to_kebab_case(&entity.snake_name)
}

fn build_scene_default_form_values(fields: &[ModelField]) -> Option<String> {
    let lines = fields
        .iter()
        .filter_map(|field| {
            let value = default_value_for_field(field);
            (value != "undefined").then(|| format!("    {}: {}", field.json_name, value))
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return None;
    }

    Some(format!("{{\n{}\n  }}", lines.join(",\n")))
}

fn build_interface_fields(entity: &ParsedEntity) -> Vec<InterfaceFieldTemplate> {
    entity
        .fields
        .iter()
        .filter(|field| !is_audit_field(&field.json_name))
        .map(|field| InterfaceFieldTemplate {
            json_name: field.json_name.clone(),
            ts_type: field.ts_type.clone(),
            is_optional: field.is_optional || !field.required,
        })
        .collect()
}

fn build_param_omit_union(entity: &ParsedEntity) -> String {
    let param_names = entity
        .param_fields
        .iter()
        .map(|field| field.json_name.as_str())
        .collect::<BTreeSet<_>>();
    let mut omit_names = AUDIT_FIELD_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();

    omit_names.extend(
        build_interface_fields(entity)
            .into_iter()
            .map(|field| field.json_name)
            .filter(|name| !param_names.contains(name.as_str())),
    );

    omit_names
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn build_identifier_validator(field: &ModelField, base: &str) -> String {
    if field.json_name == "id" || field.json_name.ends_with("Id") || field.json_name.ends_with("ID")
    {
        format!("{base}.regex(/^[a-z0-9]+$/i, \"只能包含字母和数字\")")
    } else {
        base.to_string()
    }
}

fn build_form_field_template(field: &ModelField) -> FormFieldTemplate {
    let (component_kind, span) = if field.is_json {
        ("textarea".to_string(), 24)
    } else if is_bool_field(field) {
        ("bool".to_string(), 12)
    } else if is_number_field(field) {
        ("number".to_string(), 12)
    } else if field.max_length.unwrap_or_default() > 128 || field.json_name == "remark" {
        ("textarea".to_string(), 24)
    } else {
        ("text".to_string(), 12)
    };
    let validator_mode = if field.is_json {
        None
    } else if is_number_field(field) || is_bool_field(field) {
        Some("onChange".to_string())
    } else {
        Some("onBlur".to_string())
    };
    let validator_expr = if field.is_json {
        None
    } else if is_bool_field(field) {
        Some(if field.required {
            "z.boolean(\"必须\")".to_string()
        } else {
            "z.boolean().nullish()".to_string()
        })
    } else if is_number_field(field) {
        Some(if field.required {
            "z.number(\"必须是数字\")".to_string()
        } else {
            "z.number().nullish()".to_string()
        })
    } else {
        let base = if field.required {
            "z.string(\"必须\")".to_string()
        } else {
            "z.string()".to_string()
        };
        let mut expr = build_identifier_validator(field, &base);
        if let Some(max_length) = field.max_length {
            expr.push_str(&format!(".max({max_length}, \"最多{max_length}个字符\")"));
        }
        if !field.required || field.is_optional {
            expr.push_str(".nullish()");
        }
        Some(expr)
    };

    FormFieldTemplate {
        json_name: field.json_name.clone(),
        label: field.label.clone(),
        component_kind,
        span,
        required: field.required,
        validator_mode,
        validator_expr,
        true_label: if field.json_name == "isActive" {
            "启用".to_string()
        } else {
            "是".to_string()
        },
        false_label: if field.json_name == "isActive" {
            "禁用".to_string()
        } else {
            "否".to_string()
        },
    }
}

fn build_form_fields(entity: &ParsedEntity) -> Vec<FormFieldTemplate> {
    entity
        .param_fields
        .iter()
        .map(build_form_field_template)
        .collect()
}

fn build_search_fields(entity: &ParsedEntity) -> Vec<SearchFieldTemplate> {
    let source_fields = if entity
        .search_fields
        .iter()
        .any(|field| field.json_name == "keyword")
    {
        entity
            .search_fields
            .iter()
            .filter(|field| field.json_name == "keyword")
            .cloned()
            .collect::<Vec<_>>()
    } else {
        entity.search_fields.clone()
    };

    source_fields
        .into_iter()
        .map(|field| SearchFieldTemplate {
            placeholder: if field.json_name == "keyword" {
                "关键词".to_string()
            } else {
                format!("请输入{}", field.label)
            },
            json_name: field.json_name,
            label: field.label,
        })
        .collect()
}

fn build_route_fields(entity: &ParsedEntity) -> Vec<RouteFieldTemplate> {
    let mut fields = Vec::new();
    let param_names = entity
        .param_fields
        .iter()
        .map(|field| field.json_name.as_str())
        .collect::<BTreeSet<_>>();

    if param_names.contains("id") {
        if let Some(id_field) = entity.fields.iter().find(|field| field.json_name == "id") {
            fields.push(RouteFieldTemplate {
                json_name: id_field.json_name.clone(),
                label: "ID".to_string(),
                render_kind: "none".to_string(),
                width: 120,
                align: None,
                should_cell_update: false,
                true_label: String::new(),
                false_label: String::new(),
            });
        }
    }

    fields.extend(
        entity
            .fields
            .iter()
            .filter(|field| !is_audit_field(&field.json_name))
            .map(|field| RouteFieldTemplate {
                json_name: field.json_name.clone(),
                label: field.label.clone(),
                render_kind: if is_bool_field(field) {
                    "bool".to_string()
                } else if field.is_json {
                    "json".to_string()
                } else if field.json_name == "remark" || field.max_length.unwrap_or_default() > 64 {
                    "text".to_string()
                } else {
                    "none".to_string()
                },
                width: if is_bool_field(field) || is_number_field(field) {
                    100
                } else if field.is_json
                    || field.json_name == "remark"
                    || field.max_length.unwrap_or_default() > 64
                {
                    240
                } else {
                    160
                },
                align: if is_bool_field(field) || is_number_field(field) {
                    Some("center".to_string())
                } else {
                    None
                },
                should_cell_update: false,
                true_label: if field.json_name == "isActive" {
                    "启用".to_string()
                } else {
                    "是".to_string()
                },
                false_label: if field.json_name == "isActive" {
                    "禁用".to_string()
                } else {
                    "否".to_string()
                },
            }),
    );

    for audit_name in ["createdByName", "createdAt"] {
        if entity
            .fields
            .iter()
            .any(|field| field.json_name == audit_name)
        {
            fields.push(RouteFieldTemplate {
                json_name: audit_name.to_string(),
                label: if audit_name == "createdByName" {
                    "创建人".to_string()
                } else {
                    "创建时间".to_string()
                },
                render_kind: "none".to_string(),
                width: if audit_name == "createdAt" { 180 } else { 120 },
                align: None,
                should_cell_update: true,
                true_label: String::new(),
                false_label: String::new(),
            });
        }
    }

    fields
}

fn build_context(entity: &ParsedEntity) -> Result<tera::Context, String> {
    let mut context = tera::Context::from_serialize(entity)
        .map_err(|error| format!("序列化实体失败: {error}"))?;
    let entity_file_name = frontend_entity_name(entity);

    context.insert("entity_name", &entity.name);
    context.insert("entity_file_name", &entity_file_name);
    context.insert(
        "entity_route_path",
        &format!("/_layout/{}/{}", entity.module_name, entity_file_name),
    );
    context.insert("entity_key_name", &entity.snake_name);
    context.insert("camel_entity_name", &lower_first(&entity.name));
    context.insert("crud_store_name", &format!("use{}CrudStore", entity.name));
    context.insert(
        "search_values_name",
        &format!("use{}SearchValues", entity.name),
    );
    context.insert(
        "selected_rows_name",
        &format!("use{}SelectedRows", entity.name),
    );
    context.insert(
        "operation_button_group_name",
        &format!("{}OperationButtonGroup", entity.name),
    );
    context.insert(
        "action_button_group_name",
        &format!("{}ActionButtonGroup", entity.name),
    );
    context.insert("interface_fields", &build_interface_fields(entity));
    context.insert("search_interface_fields", &entity.search_fields);
    context.insert("search_fields", &build_search_fields(entity));
    context.insert("form_fields", &build_form_fields(entity));
    context.insert("route_fields", &build_route_fields(entity));
    context.insert("param_omit_union", &build_param_omit_union(entity));
    context.insert(
        "scene_default_form_values",
        &build_scene_default_form_values(&entity.param_fields),
    );

    Ok(context)
}

fn render_template(
    tera: &tera::Tera,
    template_name: &str,
    entity: &ParsedEntity,
) -> Result<String, String> {
    let context = build_context(entity)?;
    tera.render(template_name, &context)
        .map_err(|error| format!("渲染模板 {template_name} 失败: {error}"))
}

fn output_specs(entity: &ParsedEntity) -> Vec<(String, &'static str)> {
    let entity_file_name = frontend_entity_name(entity);
    vec![
        (
            format!("apis/{}/{}.ts", entity.module_name, entity_file_name),
            "api.ts",
        ),
        (
            format!(
                "pages/_layout/{}/{}/components/basic-search.tsx",
                entity.module_name, entity_file_name
            ),
            "basic_search.tsx",
        ),
        (
            format!(
                "pages/_layout/{}/{}/components/form.tsx",
                entity.module_name, entity_file_name
            ),
            "form.tsx",
        ),
        (
            format!(
                "pages/_layout/{}/{}/helpers/index.ts",
                entity.module_name, entity_file_name
            ),
            "helpers.ts",
        ),
        (
            format!(
                "pages/_layout/{}/{}/route.tsx",
                entity.module_name, entity_file_name
            ),
            "route.tsx",
        ),
    ]
}

pub fn generate_crud_files(
    entity: &ParsedEntity,
    frontend_base_path: &str,
    overwrite: bool,
) -> Result<Vec<String>, String> {
    let base_path = Path::new(frontend_base_path);
    let specs = output_specs(entity);
    let existing_files = specs
        .iter()
        .map(|(relative_path, _)| (relative_path, base_path.join(relative_path)))
        .filter(|(_, absolute_path)| absolute_path.exists())
        .map(|(relative_path, _)| relative_path.to_string())
        .collect::<Vec<_>>();

    if !overwrite && !existing_files.is_empty() {
        return Err(format!(
            "以下文件已存在，请开启覆盖后重试:\n{}",
            existing_files.join("\n")
        ));
    }

    let tera = create_tera_engine()?;
    let mut generated_files = Vec::with_capacity(specs.len());
    for (relative_path, template_name) in specs {
        let absolute_path = base_path.join(&relative_path);
        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建目录失败 {}: {error}", parent.display()))?;
        }

        let content = render_template(&tera, template_name, entity)?;
        fs::write(&absolute_path, content)
            .map_err(|error| format!("写入文件失败 {}: {error}", absolute_path.display()))?;
        generated_files.push(path_to_string(absolute_path));
    }

    if let Some(barrel_path) = sync_api_barrel(base_path, entity)? {
        generated_files.push(barrel_path);
    }

    Ok(generated_files)
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn sync_api_barrel(base_path: &Path, entity: &ParsedEntity) -> Result<Option<String>, String> {
    let index_path = base_path.join("apis/index.ts");
    let export_line = format!(
        "export * from \"./{}/{}\";",
        entity.module_name,
        frontend_entity_name(entity)
    );

    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建目录失败 {}: {error}", parent.display()))?;
    }

    let mut content = if index_path.exists() {
        fs::read_to_string(&index_path)
            .map_err(|error| format!("读取 API 导出文件失败 {}: {error}", index_path.display()))?
    } else {
        String::new()
    };

    if content.lines().any(|line| line.trim() == export_line) {
        return Ok(None);
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&export_line);
    content.push('\n');

    fs::write(&index_path, content)
        .map_err(|error| format!("写入 API 导出文件失败 {}: {error}", index_path.display()))?;

    Ok(Some(path_to_string(index_path)))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::codegen::types::ModelField;

    fn sample_entity() -> ParsedEntity {
        ParsedEntity {
            name: "ProbationConfig".to_string(),
            snake_name: "probation_config".to_string(),
            table_name: "hr_probation_config".to_string(),
            module_name: "hr/probation".to_string(),
            fields: vec![
                ModelField {
                    name: "ID".to_string(),
                    json_name: "id".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "ID".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "CreatedAt".to_string(),
                    json_name: "createdAt".to_string(),
                    go_type: "timex.DateTime".to_string(),
                    ts_type: "string".to_string(),
                    label: "创建时间".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "CreatedByName".to_string(),
                    json_name: "createdByName".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "创建人".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "Code".to_string(),
                    json_name: "code".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "配置编码".to_string(),
                    required: true,
                    max_length: Some(64),
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "Name".to_string(),
                    json_name: "name".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "配置名称".to_string(),
                    required: true,
                    max_length: Some(128),
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "StaffScope".to_string(),
                    json_name: "staffScope".to_string(),
                    go_type: "map[string]any".to_string(),
                    ts_type: "Record<string, any>".to_string(),
                    label: "人员范围配置".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: true,
                },
                ModelField {
                    name: "IsActive".to_string(),
                    json_name: "isActive".to_string(),
                    go_type: "bool".to_string(),
                    ts_type: "boolean".to_string(),
                    label: "是否启用".to_string(),
                    required: false,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
            ],
            search_fields: vec![ModelField {
                name: "Keyword".to_string(),
                json_name: "keyword".to_string(),
                go_type: "string".to_string(),
                ts_type: "string".to_string(),
                label: "关键词".to_string(),
                required: false,
                max_length: None,
                is_optional: false,
                is_json: false,
            }],
            param_fields: vec![
                ModelField {
                    name: "ID".to_string(),
                    json_name: "id".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "ID".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "Code".to_string(),
                    json_name: "code".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "配置编码".to_string(),
                    required: true,
                    max_length: Some(64),
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "Name".to_string(),
                    json_name: "name".to_string(),
                    go_type: "string".to_string(),
                    ts_type: "string".to_string(),
                    label: "配置名称".to_string(),
                    required: true,
                    max_length: Some(128),
                    is_optional: false,
                    is_json: false,
                },
                ModelField {
                    name: "StaffScope".to_string(),
                    json_name: "staffScope".to_string(),
                    go_type: "map[string]any".to_string(),
                    ts_type: "Record<string, any>".to_string(),
                    label: "人员范围配置".to_string(),
                    required: true,
                    max_length: None,
                    is_optional: false,
                    is_json: true,
                },
                ModelField {
                    name: "IsActive".to_string(),
                    json_name: "isActive".to_string(),
                    go_type: "bool".to_string(),
                    ts_type: "boolean".to_string(),
                    label: "是否启用".to_string(),
                    required: false,
                    max_length: None,
                    is_optional: false,
                    is_json: false,
                },
            ],
            rpc_path: "hr/probation/probation_config".to_string(),
        }
    }

    #[test]
    fn test_create_tera_engine() {
        let tera = create_tera_engine().unwrap();
        assert_eq!(tera.get_template_names().count(), 5);
    }

    #[test]
    fn test_render_api_template() {
        let tera = create_tera_engine().unwrap();
        let entity = sample_entity();
        let result = render_template(&tera, "api.ts", &entity).unwrap();
        assert!(result.contains("import { API_PATH, apiClient, createApiRequest } from \"~api\";"));
        assert!(result.contains("export interface ProbationConfig extends FullAuditedEntity"));
        assert!(
            result.contains("export interface ProbationConfigParams extends Omit<ProbationConfig")
        );
        assert!(result.contains("findProbationConfigPage"));
        assert!(
            result.contains("createApiRequest(\"hr/probation/probation_config\", \"find_page\"")
        );
    }

    #[test]
    fn test_render_route_template() {
        let tera = create_tera_engine().unwrap();
        let entity = sample_entity();
        let result = render_template(&tera, "route.tsx", &entity).unwrap();
        assert!(result.contains("createFileRoute(\"/_layout/hr/probation/probation-config\")"));
        assert!(result.contains("import type { ProbationConfig } from \"~apis\";"));
        assert!(result.contains("export const Route = createFileRoute"));
        assert!(result.contains("ProbationConfigOperationButtonGroup"));
        assert!(result.contains("renderForm={() => <Form />}"));
    }

    #[test]
    fn test_generate_crud_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("sco-codegen-output-{unique}"));
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::create_dir_all(output_dir.join("apis")).expect("create api root");
        fs::write(
            output_dir.join("apis/index.ts"),
            "export * from \"./sys/app\";\n",
        )
        .expect("write barrel file");

        let generated = generate_crud_files(
            &sample_entity(),
            output_dir.to_string_lossy().as_ref(),
            false,
        )
        .expect("generator should succeed");

        assert_eq!(generated.len(), 6);
        assert!(output_dir
            .join("apis/hr/probation/probation-config.ts")
            .exists());
        assert!(output_dir
            .join("pages/_layout/hr/probation/probation-config/components/basic-search.tsx")
            .exists());
        assert!(output_dir
            .join("pages/_layout/hr/probation/probation-config/components/form.tsx")
            .exists());
        assert!(output_dir
            .join("pages/_layout/hr/probation/probation-config/helpers/index.ts")
            .exists());
        assert!(output_dir
            .join("pages/_layout/hr/probation/probation-config/route.tsx")
            .exists());
        let barrel = fs::read_to_string(output_dir.join("apis/index.ts")).expect("read barrel");
        assert!(barrel.contains("export * from \"./hr/probation/probation-config\";"));
    }

    #[test]
    fn test_generate_crud_files_rejects_existing_files_without_overwrite() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("sco-codegen-overwrite-{unique}"));
        fs::create_dir_all(output_dir.join("apis/hr/probation")).expect("create api dir");
        fs::write(
            output_dir.join("apis/hr/probation/probation-config.ts"),
            "existing",
        )
        .expect("write existing file");

        let error = generate_crud_files(
            &sample_entity(),
            output_dir.to_string_lossy().as_ref(),
            false,
        )
        .expect_err("existing files should be rejected");

        assert!(error.contains("apis/hr/probation/probation-config.ts"));
    }
}

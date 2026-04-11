use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use super::types::{
    ModelField, ParsedEntity, BASE_MODEL_FIELDS, CREATION_AUDITED_FIELDS, CREATION_TRACKED_FIELDS,
    FULL_AUDITED_FIELDS, FULL_TRACKED_FIELDS,
};

fn go_type_to_ts_type(go_type: &str) -> String {
    match go_type {
        "string" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "int" | "int32" | "int64" | "uint" | "uint32" | "uint64" => "number".to_string(),
        "time.Time" | "timex.DateTime" => "string".to_string(),
        "*string" => "MaybeNull<string>".to_string(),
        "*bool" => "MaybeNull<boolean>".to_string(),
        "*int" | "*int32" | "*int64" | "*uint" | "*uint32" | "*uint64" => {
            "MaybeNull<number>".to_string()
        }
        "*time.Time" | "*timex.DateTime" => "MaybeNull<string>".to_string(),
        "map[string]any" => "Record<string, any>".to_string(),
        "[]string" => "MaybeNull<string>[]".to_string(),
        "[]int" | "[]int32" | "[]int64" | "[]uint" | "[]uint32" | "[]uint64" => {
            "number[]".to_string()
        }
        "[]bool" => "boolean[]".to_string(),
        _ => "any".to_string(),
    }
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

fn humanize_identifier(value: &str) -> String {
    value
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut result = first.to_uppercase().to_string();
                    result.push_str(chars.as_str());
                    result
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_field_line(line: &str) -> Option<(String, String, String, String, bool, Option<usize>)> {
    let field_regex =
        Regex::new(r#"^\s*(\w+)\s+([^\s`]+)(?:\s+`([^`]*)`)?(?:\s*//.*)?\s*$"#).ok()?;
    let json_regex = Regex::new(r#"json:"([^",]+)(?:,[^"]*)?""#).ok()?;
    let label_regex = Regex::new(r#"label:"([^"]+)""#).ok()?;
    let validate_regex = Regex::new(r#"validate:"([^"]+)""#).ok()?;
    let captures = field_regex.captures(line)?;

    let field_name = captures.get(1)?.as_str().to_string();
    let go_type = captures.get(2)?.as_str().to_string();
    let tags = captures.get(3).map(|tag| tag.as_str()).unwrap_or_default();
    let json_name = json_regex
        .captures(tags)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_string())
        .unwrap_or_else(|| to_snake_case(&field_name));

    if json_name == "-" {
        return None;
    }

    let label = label_regex
        .captures(tags)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_string())
        .unwrap_or_else(|| humanize_identifier(&json_name));

    let validation_rules = validate_regex
        .captures(tags)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().split(',').collect::<Vec<_>>())
        .unwrap_or_default();

    let required = validation_rules.contains(&"required");
    let max_length = validation_rules.iter().find_map(|rule| {
        rule.strip_prefix("max=")
            .and_then(|value| value.parse::<usize>().ok())
    });

    Some((field_name, json_name, go_type, label, required, max_length))
}

fn expand_embedded_fields(embedded_name: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    if embedded_name.ends_with("BaseModel") {
        return BASE_MODEL_FIELDS.to_vec();
    }

    if embedded_name.ends_with("CreationTrackedModel") {
        return CREATION_TRACKED_FIELDS.to_vec();
    }

    if embedded_name.ends_with("FullTrackedModel") {
        return FULL_TRACKED_FIELDS.to_vec();
    }

    if embedded_name.ends_with("CreationAuditedModel") {
        return CREATION_AUDITED_FIELDS.to_vec();
    }

    if embedded_name.ends_with("FullAuditedModel") {
        return FULL_AUDITED_FIELDS.to_vec();
    }

    Vec::new()
}

fn parse_embedded_struct_name(line: &str) -> Option<String> {
    let embedded_regex = Regex::new(r#"^\s*([\w.]+)(?:\s+`[^`]*`)?(?:\s*//.*)?\s*$"#).ok()?;
    let captures = embedded_regex.captures(line)?;
    captures.get(1).map(|capture| capture.as_str().to_string())
}

fn build_model_field(
    name: String,
    json_name: String,
    go_type: String,
    label: String,
    required: bool,
    max_length: Option<usize>,
) -> ModelField {
    let ts_type = go_type_to_ts_type(&go_type);
    let is_optional = go_type.starts_with('*');
    let is_json = go_type == "map[string]any";

    ModelField {
        name,
        json_name,
        go_type,
        ts_type,
        label,
        required,
        max_length,
        is_optional,
        is_json,
    }
}

fn parse_struct_fields_optional(
    content: &str,
    struct_name: &str,
) -> Result<Option<Vec<ModelField>>, String> {
    let struct_regex = Regex::new(&format!(
        r#"^\s*type\s+{}\s+struct\s*\{{"#,
        regex::escape(struct_name)
    ))
    .map_err(|error| format!("构建 struct 匹配失败: {error}"))?;
    let mut in_struct = false;
    let mut fields = Vec::new();

    for line in content.lines() {
        if !in_struct {
            if struct_regex.is_match(line) {
                in_struct = true;
            }
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if trimmed == "}" {
            return Ok(Some(fields));
        }

        if let Some(embedded_name) = parse_embedded_struct_name(line) {
            for (name, json_name, ts_type) in expand_embedded_fields(&embedded_name) {
                fields.push(ModelField {
                    name: name.to_string(),
                    json_name: json_name.to_string(),
                    go_type: ts_type.to_string(),
                    ts_type: ts_type.to_string(),
                    label: humanize_identifier(name),
                    required: false,
                    max_length: None,
                    is_optional: ts_type.starts_with("MaybeNull<"),
                    is_json: false,
                });
            }
            continue;
        }

        if let Some((name, json_name, go_type, label, required, max_length)) =
            parse_field_line(line)
        {
            fields.push(build_model_field(
                name, json_name, go_type, label, required, max_length,
            ));
        }
    }

    Ok(if in_struct { Some(fields) } else { None })
}

fn parse_struct_fields(content: &str, struct_name: &str) -> Result<Vec<ModelField>, String> {
    parse_struct_fields_optional(content, struct_name)?
        .ok_or_else(|| format!("未找到 struct {struct_name} 定义"))
}

pub fn parse_model_file(content: &str, struct_name: &str) -> Result<Vec<ModelField>, String> {
    parse_struct_fields(content, struct_name)
}

pub fn parse_payload_file(
    content: &str,
    entity_name: &str,
) -> Result<(Vec<ModelField>, Vec<ModelField>), String> {
    let search_fields =
        parse_struct_fields_optional(content, &format!("{entity_name}Search"))?.unwrap_or_default();
    let param_fields =
        parse_struct_fields_optional(content, &format!("{entity_name}Params"))?.unwrap_or_default();

    Ok((search_fields, param_fields))
}

pub fn extract_rpc_path(content: &str) -> Option<String> {
    let config_path_regex = Regex::new(r#"Path\s*:\s*"([^"]+)""#).ok()?;
    if let Some(captures) = config_path_regex.captures(content) {
        return captures.get(1).map(|capture| capture.as_str().to_string());
    }

    let rpc_resource_regex = Regex::new(r#"NewRPCResource\("([^"]+)""#).ok()?;
    if let Some(captures) = rpc_resource_regex.captures(content) {
        return captures.get(1).map(|capture| capture.as_str().to_string());
    }

    let method_path_regex =
        Regex::new(r#"(?s)Path\(\)\s+string\s*\{.*?return\s+"([^"]+)""#).ok()?;
    method_path_regex
        .captures(content)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_string())
}

fn extract_module_and_entity(resource_path: &str) -> Result<(String, String), String> {
    let normalized = resource_path.replace('\\', "/");
    let regex = Regex::new(r#"internal/(.+)/resource/([^/]+)\.go$"#)
        .map_err(|error| format!("构建路径匹配失败: {error}"))?;
    let captures = regex.captures(&normalized).ok_or_else(|| {
        "无法从路径推导模块名，期望格式: internal/{module...}/resource/{entity}.go".to_string()
    })?;

    let module_name = captures
        .get(1)
        .map(|capture| capture.as_str().to_string())
        .ok_or_else(|| "无法读取模块名".to_string())?;
    let entity_name = captures
        .get(2)
        .map(|capture| capture.as_str().to_string())
        .ok_or_else(|| "无法读取实体名".to_string())?;

    Ok((module_name, entity_name))
}

fn extract_table_name(content: &str) -> Option<String> {
    let table_regex = Regex::new(r#"table:([^,`"]+)"#).ok()?;
    table_regex
        .captures(content)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_string())
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut value = first.to_uppercase().to_string();
                    value.push_str(chars.as_str());
                    value
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn derive_peer_path(resource_path: &Path, peer_dir: &str) -> Result<PathBuf, String> {
    let (_, entity_name) = extract_module_and_entity(resource_path.to_string_lossy().as_ref())?;
    let root = resource_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("无法推导 resource 根目录: {}", resource_path.display()))?;
    Ok(root.join(peer_dir).join(format!("{entity_name}.go")))
}

pub fn parse_go_resource(resource_path: &str) -> Result<ParsedEntity, String> {
    let resource_path = Path::new(resource_path);
    let (module_name, entity_name) =
        extract_module_and_entity(resource_path.to_string_lossy().as_ref())?;
    let resource_content = fs::read_to_string(resource_path).map_err(|error| {
        format!(
            "读取 Resource 文件失败 {}: {error}",
            resource_path.display()
        )
    })?;
    let rpc_path = extract_rpc_path(&resource_content)
        .ok_or_else(|| format!("未找到 Resource Path 配置: {}", resource_path.display()))?;
    let model_path = derive_peer_path(resource_path, "model")?;
    let payload_path = derive_peer_path(resource_path, "payload")?;

    if !model_path.exists() {
        return Err(format!("Model 文件不存在: {}", model_path.display()));
    }

    let model_content = fs::read_to_string(&model_path)
        .map_err(|error| format!("读取 Model 文件失败 {}: {error}", model_path.display()))?;
    let fields = parse_model_file(&model_content, &to_pascal_case(&entity_name))?;
    let table_name = extract_table_name(&model_content)
        .unwrap_or_else(|| format!("{}_{}", module_name.replace('/', "_"), entity_name));

    let (search_fields, param_fields) = match fs::read_to_string(&payload_path) {
        Ok(payload_content) => parse_payload_file(&payload_content, &to_pascal_case(&entity_name))?,
        Err(_) => (Vec::new(), Vec::new()),
    };

    Ok(ParsedEntity {
        name: to_pascal_case(&entity_name),
        snake_name: entity_name.clone(),
        table_name,
        module_name,
        fields,
        search_fields,
        param_fields,
        rpc_path,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn test_go_type_to_ts_type() {
        assert_eq!(go_type_to_ts_type("string"), "string");
        assert_eq!(go_type_to_ts_type("*string"), "MaybeNull<string>");
        assert_eq!(go_type_to_ts_type("int64"), "number");
        assert_eq!(go_type_to_ts_type("map[string]any"), "Record<string, any>");
        assert_eq!(go_type_to_ts_type("CustomType"), "any");
    }

    #[test]
    fn test_parse_field_line() {
        let line = r#"    Name string `json:"name" label:"名称" validate:"required,max=100"`"#;
        let result = parse_field_line(line);
        assert!(result.is_some());
        let (_name, json_name, _go_type, label, required, max_len) =
            result.expect("field should parse");
        assert_eq!(json_name, "name");
        assert_eq!(label, "名称");
        assert!(required);
        assert_eq!(max_len, Some(100));
    }

    #[test]
    fn test_extract_module_and_entity() {
        let (module, entity) =
            extract_module_and_entity("/path/to/project/internal/sys/resource/app.go").unwrap();
        assert_eq!(module, "sys");
        assert_eq!(entity, "app");
    }

    #[test]
    fn test_parse_model_file() {
        let content = r#"
type ProbationConfig struct {
    orm.BaseModel `bun:"table:hr_probation_config,alias:hpc"`
    orm.CreationAuditedModel
    Code string `json:"code" label:"编码" validate:"required"`
}
"#;
        let fields = parse_model_file(content, "ProbationConfig").unwrap();
        assert!(fields.iter().any(|field| field.json_name == "id"));
        assert!(fields.iter().any(|field| field.json_name == "createdBy"));
        assert!(fields.iter().any(|field| field.json_name == "code"));
    }

    #[test]
    fn test_parse_payload_file() {
        let content = r#"
type AppSearch struct {
    Keyword string `json:"keyword" label:"关键词"`
}

type AppParams struct {
    Name string `json:"name" label:"名称" validate:"required,max=100"`
    Enabled bool `json:"enabled" label:"启用"`
}
"#;
        let (search_fields, param_fields) = parse_payload_file(content, "App").unwrap();
        assert_eq!(search_fields.len(), 1);
        assert_eq!(search_fields[0].json_name, "keyword");
        assert_eq!(param_fields.len(), 2);
        assert!(param_fields[0].required);
        assert_eq!(param_fields[0].max_length, Some(100));
    }

    #[test]
    fn test_extract_rpc_path() {
        let content = r#"
var appResource = resource.NewResource[model.App, payload.AppSearch, payload.AppParams](
    resource.Config{
        Path: "smp/sys/app",
    },
)
"#;
        assert_eq!(extract_rpc_path(content), Some("smp/sys/app".to_string()));
    }

    #[test]
    fn test_extract_rpc_path_from_new_rpc_resource() {
        let content = r#"
func NewProbationConfigResource() api.Resource {
    return &ProbationConfigResource{
        Resource: api.NewRPCResource("hr/probation/probation_config"),
    }
}
"#;
        assert_eq!(
            extract_rpc_path(content),
            Some("hr/probation/probation_config".to_string())
        );
    }

    #[test]
    fn test_extract_module_and_entity_supports_nested_paths() {
        let (module, entity) = extract_module_and_entity(
            "/path/to/project/internal/hr/probation/resource/probation_config.go",
        )
        .unwrap();
        assert_eq!(module, "hr/probation");
        assert_eq!(entity, "probation_config");
    }

    #[test]
    fn test_parse_go_resource() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sco-codegen-parser-{unique}"));
        let resource_dir = root.join("internal/hr/probation/resource");
        let model_dir = root.join("internal/hr/probation/model");
        let payload_dir = root.join("internal/hr/probation/payload");

        fs::create_dir_all(&resource_dir).expect("create resource dir");
        fs::create_dir_all(&model_dir).expect("create model dir");
        fs::create_dir_all(&payload_dir).expect("create payload dir");

        fs::write(
            resource_dir.join("probation_config.go"),
            r#"
func NewProbationConfigResource() api.Resource {
    return &ProbationConfigResource{
        Resource: api.NewRPCResource("hr/probation/probation_config"),
    }
}
"#,
        )
        .expect("write resource");
        fs::write(
            model_dir.join("probation_config.go"),
            r#"
type ProbationConfig struct {
    orm.BaseModel `bun:"table:hr_probation_config,alias:hpc"`
    orm.CreationAuditedModel
    Name string `json:"name" label:"名称" validate:"required"`
}
"#,
        )
        .expect("write model");
        fs::write(
            payload_dir.join("probation_config.go"),
            r#"
type ProbationConfigSearch struct {
    Keyword string `json:"keyword" label:"关键词"`
}

type ProbationConfigParams struct {
    Name string `json:"name" label:"名称" validate:"required"`
}
"#,
        )
        .expect("write payload");

        let entity = parse_go_resource(
            resource_dir
                .join("probation_config.go")
                .to_string_lossy()
                .as_ref(),
        )
        .expect("resource should parse");

        assert_eq!(entity.name, "ProbationConfig");
        assert_eq!(entity.module_name, "hr/probation");
        assert_eq!(entity.table_name, "hr_probation_config");
        assert_eq!(entity.rpc_path, "hr/probation/probation_config");
        assert_eq!(entity.search_fields.len(), 1);
        assert_eq!(entity.param_fields.len(), 1);
        assert!(entity
            .fields
            .iter()
            .any(|field| field.json_name == "createdBy"));
        assert!(entity.fields.iter().any(|field| field.json_name == "name"));
    }
}

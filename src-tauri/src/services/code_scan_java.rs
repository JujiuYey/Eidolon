//! vef-java 框架的源码扫描器：静态解析 Spring MVC Controller 与
//! vef Form，生成接口清单 JSON（与 vef-go 的 `VEF_EXPORT_API` 清单
//! 同构的「原始事实」，展示映射由前端适配器完成）。
//!
//! Java 版 vef 没有清单导出器，也没有 swagger/OpenAPI；本项目代码
//! 风格高度规约化（类级 `@RequestMapping` + 方法级 mapping 注解、
//! kebab-case 路径），因此用轻量行级解析而非完整语法树。解析失败
//! 的文件跳过并记入 warnings，不阻断整体导出。
//!
//! 只读不执行：扫描仅遍历 `src/main/java` 下的 `.java` 文本文件。

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

// ============================================================
// 清单数据模型（输出 JSON 的形状，前端 vef-java 适配器消费）
// ============================================================

#[derive(Serialize)]
struct JavaManifest {
    framework: String,
    controllers: Vec<ControllerDef>,
    forms: HashMap<String, FormDef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ControllerDef {
    /// 类名，如 `DepartmentController`
    name: String,
    /// 业务模块，取包名 `com.dhyx.mrqcm.<module>.controller` 的段
    module: String,
    /// 类级 @RequestMapping 路径，如 `/system/department`
    base_path: String,
    /// 是否继承 AbstractBaseController（继承则展开通用端点）
    extends_base: bool,
    /// 泛型第 4 参数（UpdateForm），用于 create/modify 的请求体骨架
    update_form: Option<String>,
    /// 泛型第 3 参数（SearchForm），用于查询类端点的 query 骨架
    search_form: Option<String>,
    endpoints: Vec<EndpointDef>,
}

#[derive(Serialize, Clone)]
struct EndpointDef {
    /// 方法名；基类继承端点用约定名（fetchPage 等）
    name: String,
    method: String,
    /// 相对 base_path 的子路径，如 `/page`、`/department-tree`
    path: String,
    /// `declared`（源码声明）或 `base`（基类继承展开）
    source: &'static str,
    /// @RequestBody 类型（简单名），batch 端点由前端包装为数组
    body_form: Option<String>,
    query_params: Vec<QueryParamDef>,
}

#[derive(Serialize, Clone)]
struct QueryParamDef {
    name: String,
    required: bool,
}

#[derive(Serialize)]
struct FormDef {
    fields: Vec<FieldDef>,
}

#[derive(Serialize)]
struct FieldDef {
    name: String,
    /// Java 声明类型（String/Integer/枚举名等）
    field_type: String,
    required: bool,
    /// @Comment 中文名
    label: Option<String>,
}

/// vef AbstractBaseController 提供的通用端点。
/// 子类声明了同 method+path 的端点时以声明为准（override）。
const BASE_ENDPOINTS: &[(&str, &str, &str)] = &[
    ("fetchOne", "GET", "/"),
    ("fetchList", "GET", "/list"),
    ("fetchPage", "GET", "/page"),
    ("create", "POST", "/"),
    ("modify", "PUT", "/"),
    ("delete", "DELETE", "/"),
    ("batchCreate", "POST", "/batch"),
    ("batchModify", "PUT", "/batch"),
    ("batchDelete", "DELETE", "/batch"),
    ("fetchMeta", "GET", "/meta"),
    ("fetchOptions", "GET", "/options"),
    ("export", "GET", "/export"),
    ("import", "POST", "/import"),
    ("importData", "POST", "/import/data"),
    ("importTemplate", "GET", "/import/template"),
    ("importPreview", "POST", "/import/preview"),
];

pub fn scan(repo_path: &Path) -> Result<String, String> {
    let java_root = repo_path.join("src").join("main").join("java");
    if !java_root.is_dir() {
        return Err(format!(
            "未找到 {}，请确认这是 vef-java 项目的仓库根目录",
            java_root.display()
        ));
    }

    let mut controllers = Vec::new();
    let mut forms: HashMap<String, FormDef> = HashMap::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(&java_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("java") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                warnings.push(format!("读取 {} 失败: {error}", path.display()));
                continue;
            }
        };

        if content.contains("*Controller")
            || path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .ends_with("Controller")
        {
            match parse_controller(&content) {
                Some(controller) => controllers.push(controller),
                None => warnings.push(format!("未能解析 {}", path.display())),
            }
        } else if content.contains("AbstractUpdateForm") || content.contains("AbstractSearchForm") {
            if let Some((name, form)) = parse_form(&content) {
                forms.insert(name, form);
            }
        }
    }

    controllers.sort_by(|a, b| a.base_path.cmp(&b.base_path));
    let manifest = JavaManifest {
        framework: "vef-java".to_string(),
        controllers,
        forms,
        warnings,
    };

    serde_json::to_string_pretty(&manifest).map_err(|error| format!("序列化清单失败: {error}"))
}

// ============================================================
// Controller 解析
// ============================================================

fn parse_controller(content: &str) -> Option<ControllerDef> {
    let package = Regex::new(r"(?m)^package\s+([\w.]+);").unwrap();
    let class_decl =
        Regex::new(r"(?m)public\s+class\s+(\w+Controller)(?:\s+extends\s+(\w+))?(?:\s*<([^>]*)>)?")
            .unwrap();

    let class = class_decl.captures(content)?;
    let class_name = class.get(1)?.as_str().to_string();
    let super_name = class.get(2).map(|m| m.as_str().to_string());
    let generics = class.get(3).map(|m| m.as_str().to_string());

    let module = package
        .captures(content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .and_then(|pkg| {
            let segments: Vec<&str> = pkg.split('.').collect();
            // com.dhyx.mrqcm.<module>.controller → module；其余归入 common
            if segments.len() >= 2 && segments[segments.len() - 1] == "controller" {
                Some(segments[segments.len() - 2].to_string())
            } else {
                Some("common".to_string())
            }
        })
        .unwrap_or_else(|| "common".to_string());

    let base_path = extract_class_mapping(content, class.get(0)?.start())
        .unwrap_or_else(|| format!("/{}", kebab(&class_name.trim_end_matches("Controller"))));

    let (extends_base, search_form, update_form) = match super_name.as_deref() {
        Some("AbstractBaseController") => {
            let types = generics
                .map(|raw| {
                    raw.split(',')
                        .map(|item| simple_type(item.trim()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let search = types.get(2).cloned().filter(|name| name != "?");
            let update = types.get(3).cloned().filter(|name| name != "?");
            (true, search, update)
        }
        _ => (false, None, None),
    };

    let declared = parse_declared_endpoints(content, class.get(0)?.end());

    let mut endpoints = declared;
    if extends_base {
        for (name, method, path) in BASE_ENDPOINTS {
            let overridden = endpoints
                .iter()
                .any(|endpoint| endpoint.method == *method && endpoint.path == *path);
            if overridden {
                continue;
            }
            endpoints.push(EndpointDef {
                name: name.to_string(),
                method: method.to_string(),
                path: path.to_string(),
                source: "base",
                body_form: base_body_form(name, update_form.as_deref()),
                query_params: base_query_params(name),
            });
        }
    }

    endpoints.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));

    if endpoints.is_empty() {
        return None;
    }

    Some(ControllerDef {
        name: class_name,
        module,
        base_path,
        extends_base,
        update_form,
        search_form,
        endpoints,
    })
}

/// 基类端点中需要请求体 / 查询参数的约定
fn base_body_form(name: &str, update_form: Option<&str>) -> Option<String> {
    match name {
        "create" | "modify" | "batchCreate" | "batchModify" | "import" | "importData"
        | "importPreview" => update_form.map(|form| form.to_string()),
        _ => None,
    }
}

fn base_query_params(name: &str) -> Vec<QueryParamDef> {
    match name {
        "fetchOne" | "delete" => vec![QueryParamDef {
            name: "id".to_string(),
            required: true,
        }],
        "batchDelete" => vec![QueryParamDef {
            name: "ids".to_string(),
            required: true,
        }],
        _ => Vec::new(),
    }
}

/// 类级 @RequestMapping：取 class 声明前最后一个 mapping 注解里的
/// 第一个字符串字面量（value 或 path 属性，剥掉 produces 等）
fn extract_class_mapping(content: &str, class_start: usize) -> Option<String> {
    let head = &content[..class_start];
    let mapping = Regex::new(r#"@RequestMapping\s*(?:\(([^;]*?)\))?"#).unwrap();
    let caps = mapping.captures(head)?;
    let args = caps
        .get(1)
        .map(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    let literal = Regex::new(r#""([^"]*)""#).unwrap();
    literal
        .captures(&args)
        .map(|c| c.get(1).unwrap().as_str().to_string())
}

/// class 体里声明的方法级端点
fn parse_declared_endpoints(content: &str, body_start: usize) -> Vec<EndpointDef> {
    let body = &content[body_start..];
    // 参数列表贪婪匹配到最后一个 `)`（@RequestParam(...) 自带嵌套括号），
    // 以 `;`/`{` 为界防止吃进方法体
    let method_mapping = Regex::new(
        r#"@(Get|Post|Put|Delete|Patch)Mapping\s*(?:\(([^;{]*?)\))?\s*(?:@\w+(?:\([^;{]*?\))?\s*)*(?:public|protected)\s+[\w<>,\[\]\s.?]+?\s+(\w+)\s*\(([^;{]*)\)"#,
    )
    .unwrap();

    let request_param_paren =
        Regex::new(r#"@RequestParam\(([^)]*)\)\s*(?:[\w.<>\[\]]+\s+)?([\w.<>\[\]]+)\s+(\w+)"#)
            .unwrap();
    let request_param_bare = Regex::new(r"@RequestParam\s+([\w.<>\[\]]+)\s+(\w+)").unwrap();
    let request_body = Regex::new(r"@RequestBody\s+([\w.]+)").unwrap();

    let literal = Regex::new(r#""([^"]*)""#).unwrap();
    let attr_name = Regex::new(r#"name\s*=\s*"([^"]*)""#).unwrap();
    let attr_required = Regex::new(r"required\s*=\s*(\w+)").unwrap();

    let mut endpoints = Vec::new();
    for caps in method_mapping.captures_iter(body) {
        let http_method = caps.get(1).unwrap().as_str().to_ascii_uppercase();
        let args = caps
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        let name = caps.get(3).unwrap().as_str().to_string();
        let params_text = caps
            .get(4)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string();

        let path = match literal.captures(&args) {
            Some(caps) => caps.get(1).unwrap().as_str().to_string(),
            None => "/".to_string(),
        };

        let body_form = request_body
            .captures(&params_text)
            .map(|caps| simple_type(caps.get(1).unwrap().as_str()));

        let mut query_params = Vec::new();
        for param in request_param_paren.captures_iter(&params_text) {
            let attrs = param.get(1).map(|m| m.as_str()).unwrap_or_default();
            let declared_name = attr_name
                .captures(attrs)
                .map(|caps| caps.get(1).unwrap().as_str().to_string())
                .or_else(|| {
                    literal
                        .captures(attrs)
                        .map(|caps| caps.get(1).unwrap().as_str().to_string())
                });
            let required = attr_required
                .captures(attrs)
                .map(|caps| caps.get(1).unwrap().as_str() == "true");
            let variable = param.get(3).unwrap().as_str().to_string();
            query_params.push(QueryParamDef {
                name: declared_name.unwrap_or(variable),
                // Spring 语义：@RequestParam 默认必填
                required: required.unwrap_or(true),
            });
        }
        for param in request_param_bare.captures_iter(&params_text) {
            let variable = param.get(2).unwrap().as_str().to_string();
            query_params.push(QueryParamDef {
                name: variable,
                required: true,
            });
        }

        endpoints.push(EndpointDef {
            name,
            method: http_method,
            path,
            source: "declared",
            body_form,
            query_params,
        });
    }

    endpoints
}

// ============================================================
// Form 解析（UpdateForm / SearchForm 字段表）
// ============================================================

fn parse_form(content: &str) -> Option<(String, FormDef)> {
    let class_decl =
        Regex::new(r"(?m)public\s+(?:abstract\s+)?class\s+(\w+)\s+extends\s+(\w+)").unwrap();
    let caps = class_decl.captures(content)?;
    let name = caps.get(1)?.as_str().to_string();
    let super_name = caps.get(2)?.as_str();

    // 只关心 vef 表单基类的具体子类，抽象基类本身跳过
    if !(super_name.contains("UpdateForm") || super_name.contains("SearchForm"))
        || name.starts_with("Abstract")
    {
        return None;
    }

    let comment = Regex::new(r#"@Comment\s*\(\s*"([^"]*)""#).unwrap();
    let field_decl = Regex::new(
        r"(?m)^\s*(?:private|protected|public)\s+(?:final\s+)?([\w<>,\[\]\s.]+?)\s+(\w+)\s*(?:=[^;]*)?;",
    )
    .unwrap();

    let mut fields = Vec::new();
    let mut pending_comment: Option<String> = None;
    let mut pending_required = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            if let Some(caps) = comment.captures(trimmed) {
                pending_comment = Some(caps.get(1).unwrap().as_str().to_string());
            }
            if trimmed.starts_with("@NotNull")
                || trimmed.starts_with("@NotBlank")
                || trimmed.starts_with("@NotEmpty")
            {
                pending_required = true;
            }
            continue;
        }

        if let Some(caps) = field_decl.captures(line) {
            let field_type = caps.get(1).unwrap().as_str().trim().to_string();
            let field_name = caps.get(2).unwrap().as_str().to_string();
            let is_static = false;
            if !is_static && !field_type.starts_with("static") {
                fields.push(FieldDef {
                    name: field_name,
                    field_type,
                    required: pending_required,
                    label: pending_comment.take(),
                });
                pending_required = false;
            }
        }
    }

    Some((name, FormDef { fields }))
}

// ============================================================
// 工具
// ============================================================

/// 泛型参数 `com.dhyx.mrqcm.system.form.update.DepartmentUpdateForm` → `DepartmentUpdateForm`
fn simple_type(raw: &str) -> String {
    raw.rsplit('.').next().unwrap_or(raw).trim().to_string()
}

/// CamelCase → kebab-case（BasePath 缺省时兜底）
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if index > 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROLLER_SOURCE: &str = r#"
package com.dhyx.mrqcm.system.controller;

@RestController
@RequestMapping(value = "/system/department", produces = MediaType.APPLICATION_JSON_VALUE)
public class DepartmentController extends AbstractBaseController<Department, DepartmentService,
        DepartmentSearchForm, DepartmentUpdateForm, DepartmentController> {

    @GetMapping("/department-tree")
    public JsonResult<List<Department>> fetchDepartmentTree(@RequestParam(required = false) String tenantId) {
        return null;
    }

    @PostMapping("/batch-move")
    public JsonResult<Void> batchMove(@RequestBody DepartmentMoveForm moveForm) {
        return null;
    }
}
"#;

    const EXTERNAL_CONTROLLER: &str = r#"
package com.dhyx.mrqcm.api.controller;

@RestController
@RequestMapping(value = "/external-api", produces = MediaType.APPLICATION_JSON_VALUE)
public class ExternalApiController {

    @PostMapping("/business-data")
    public JsonResult<Void> syncBusinessData(@RequestBody ExternalRequestDto externalRequestDto) {
        return null;
    }

    @GetMapping("/medical-record-file")
    @RequestLimiting
    public JsonResult<MedicalRecordFileDto> fetchMedicalRecordFile(@RequestParam String patientId) {
        return null;
    }
}
"#;

    const UPDATE_FORM: &str = r#"
package com.dhyx.mrqcm.system.form.update;

public class DepartmentUpdateForm extends AbstractUpdateForm {

    @NotBlank
    @Size(max = 32)
    @Comment("部门名称")
    private String name;

    @NotNull
    @Comment("部门类型")
    private DepartmentType type;

    private String remark;
}
"#;

    const SEARCH_FORM: &str = r#"
package com.dhyx.mrqcm.system.form.search;

public class DepartmentSearchForm extends AbstractSearchForm {

    @SearchCondition(strategy = ConditionType.EQUALS)
    private SwitchStatus status;

    @SearchCondition(strategy = ConditionType.LIKE)
    @Comment("部门名称")
    private String name;
}
"#;

    #[test]
    fn parses_controller_with_base_inheritance() {
        let controller = parse_controller(CONTROLLER_SOURCE).expect("controller");

        assert_eq!(controller.module, "system");
        assert_eq!(controller.base_path, "/system/department");
        assert!(controller.extends_base);
        assert_eq!(
            controller.search_form.as_deref(),
            Some("DepartmentSearchForm")
        );
        assert_eq!(
            controller.update_form.as_deref(),
            Some("DepartmentUpdateForm")
        );

        // 2 个声明端点 + 16 个基类端点
        assert_eq!(controller.endpoints.len(), 18);

        let tree = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/department-tree")
            .expect("declared endpoint");
        assert_eq!(tree.method, "GET");
        assert_eq!(tree.source, "declared");
        assert_eq!(tree.query_params.len(), 1);
        assert_eq!(tree.query_params[0].name, "tenantId");
        assert!(!tree.query_params[0].required);

        let move_endpoint = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/batch-move")
            .expect("declared post endpoint");
        assert_eq!(move_endpoint.method, "POST");
        assert_eq!(
            move_endpoint.body_form.as_deref(),
            Some("DepartmentMoveForm")
        );

        let page = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "fetchPage")
            .expect("base fetchPage");
        assert_eq!((page.method.as_str(), page.path.as_str()), ("GET", "/page"));
        assert_eq!(page.source, "base");

        let create = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "create")
            .expect("base create");
        assert_eq!(create.body_form.as_deref(), Some("DepartmentUpdateForm"));
    }

    #[test]
    fn parses_standalone_controller_without_base() {
        let controller = parse_controller(EXTERNAL_CONTROLLER).expect("controller");

        assert_eq!(controller.module, "api");
        assert_eq!(controller.base_path, "/external-api");
        assert!(!controller.extends_base);
        assert_eq!(controller.endpoints.len(), 2);

        let sync = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/business-data")
            .expect("business-data");
        assert_eq!(sync.method, "POST");
        assert_eq!(sync.body_form.as_deref(), Some("ExternalRequestDto"));

        // @RequestLimiting 注解夹在 mapping 与签名之间也能解析
        let file = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/medical-record-file")
            .expect("medical-record-file");
        assert_eq!(file.query_params[0].name, "patientId");
        assert!(file.query_params[0].required);
    }

    #[test]
    fn parses_update_and_search_form_fields() {
        let (name, form) = parse_form(UPDATE_FORM).expect("update form");
        assert_eq!(name, "DepartmentUpdateForm");
        assert_eq!(form.fields.len(), 3);
        assert_eq!(form.fields[0].name, "name");
        assert!(form.fields[0].required);
        assert_eq!(form.fields[0].label.as_deref(), Some("部门名称"));
        assert_eq!(form.fields[1].field_type, "DepartmentType");
        assert!(!form.fields[2].required);

        let (name, form) = parse_form(SEARCH_FORM).expect("search form");
        assert_eq!(name, "DepartmentSearchForm");
        assert_eq!(form.fields.len(), 2);
        assert_eq!(form.fields[1].label.as_deref(), Some("部门名称"));
        assert!(!form.fields[0].required, "search form 字段默认可选");
    }

    #[test]
    fn kebab_case_conversion() {
        assert_eq!(kebab("Department"), "department");
        assert_eq!(kebab("GroupingResult"), "grouping-result");
    }

    #[test]
    fn scan_rejects_missing_java_root() {
        let error = scan(Path::new("/nonexistent/mrqcm")).expect_err("missing root");
        assert!(error.contains("未找到"), "got {error}");
    }

    #[test]
    fn scan_real_repo_if_present() {
        // 开发机上有真实仓库时做端到端冒烟；CI/其他机器跳过
        let repo = Path::new("/Users/jujiuyey/dhyxtech-work/mrqcm/mrqcm-server");
        if !repo.is_dir() {
            return;
        }
        let json = scan(repo).expect("scan real repo");
        let manifest: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let controllers = manifest["controllers"].as_array().expect("controllers");
        assert!(controllers.len() > 100, "got {}", controllers.len());
        let total: usize = controllers
            .iter()
            .map(|controller| {
                controller["endpoints"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0)
            })
            .sum();
        assert!(total > 1500, "got {total}");
    }
}

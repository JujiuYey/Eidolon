//! Java 源码扫描器：静态解析 Spring MVC Controller 与表单类，
//! 生成接口清单 JSON（与 vef-go 的 `VEF_EXPORT_API` 清单同构的
//! 「原始事实」，展示映射由前端适配器完成）。
//!
//! 支持两种 dhyx 系框架约定：
//! - vef（mrqcm 等）：`AbstractBaseController`，16 个通用端点，RPC 风格路径
//! - dhyx framework（peis 等）：`AbstractFrameworkController`，10 个通用端点，
//!   REST 风格路径（`{id}` 路径参数、`/dict` 字典），swagger 注解
//!   `@Api`/`@ApiOperation` 提供中文标签
//!
//! 两种约定的泛型第 3/4 位都是 SearchForm/UpdateForm。解析采用括号
//! 平衡扫描而非单一正则，以容忍方法上夹带的嵌套注解
//! （`@ApiImplicitParams(value = {@ApiImplicitParam(...)})`）。
//! 解析失败的文件跳过并记入 warnings，不阻断整体导出。
//! 只读不执行：仅遍历 `src/main/java` 下的 `.java` 文本文件。

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

// ============================================================
// 清单数据模型（输出 JSON 的形状，前端适配器消费）
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
    /// 业务模块，取包名 `<base>.<module>.controller` 的段
    module: String,
    /// 类级 @RequestMapping 路径，如 `/system/department`
    base_path: String,
    /// swagger @Api(tags) 中文标签（vef 项目为空）
    label: Option<String>,
    /// 基类约定：`vef` | `dhyx` | `none`
    framework_kind: &'static str,
    /// 泛型第 4 参数（UpdateForm），用于写操作的请求体骨架
    update_form: Option<String>,
    /// 泛型第 3 参数（SearchForm），用于查询类端点的 query 骨架
    search_form: Option<String>,
    endpoints: Vec<EndpointDef>,
}

#[derive(Serialize, Clone)]
struct EndpointDef {
    /// 方法名；基类继承端点用约定名（fetchPage / page 等）
    name: String,
    /// swagger @ApiOperation(value) 中文标签
    label: Option<String>,
    method: String,
    /// 相对 base_path 的子路径，如 `/page`、`/department-tree`、`/{id}`
    path: String,
    /// `declared`（源码声明）或 `base`（基类继承展开）
    source: &'static str,
    /// @RequestBody 类型（简单名）；数组包裹由前端按端点约定处理
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
    /// 中文名（@Comment / @ApiModelProperty）
    label: Option<String>,
}

/// 基类通用端点约定：名称、方法、子路径、是否以 UpdateForm 为请求体、
/// 自带 query 参数（SearchForm 与分页由前端按端点名注入）。
struct BaseEndpointSpec {
    name: &'static str,
    method: &'static str,
    path: &'static str,
    body: bool,
    query: &'static [(&'static str, bool)],
}

const NO_QUERY: &[(&str, bool)] = &[];

/// vef AbstractBaseController（16 端点）
const VEF_BASE_ENDPOINTS: &[BaseEndpointSpec] = &[
    BaseEndpointSpec {
        name: "fetchOne",
        method: "GET",
        path: "/",
        body: false,
        query: &[("id", true)],
    },
    BaseEndpointSpec {
        name: "fetchList",
        method: "GET",
        path: "/list",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "fetchPage",
        method: "GET",
        path: "/page",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "create",
        method: "POST",
        path: "/",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "modify",
        method: "PUT",
        path: "/",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "delete",
        method: "DELETE",
        path: "/",
        body: false,
        query: &[("id", true)],
    },
    BaseEndpointSpec {
        name: "batchCreate",
        method: "POST",
        path: "/batch",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "batchModify",
        method: "PUT",
        path: "/batch",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "batchDelete",
        method: "DELETE",
        path: "/batch",
        body: false,
        query: &[("ids", true)],
    },
    BaseEndpointSpec {
        name: "fetchMeta",
        method: "GET",
        path: "/meta",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "fetchOptions",
        method: "GET",
        path: "/options",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "export",
        method: "GET",
        path: "/export",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "import",
        method: "POST",
        path: "/import",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "importData",
        method: "POST",
        path: "/import/data",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "importTemplate",
        method: "GET",
        path: "/import/template",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "importPreview",
        method: "POST",
        path: "/import/preview",
        body: true,
        query: NO_QUERY,
    },
];

/// dhyx AbstractFrameworkController（10 端点）。
/// 分页 query（page/size）与 SearchForm 字段由前端注入默认值。
const DHYX_BASE_ENDPOINTS: &[BaseEndpointSpec] = &[
    BaseEndpointSpec {
        name: "get",
        method: "GET",
        path: "/{id}",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "page",
        method: "GET",
        path: "/",
        body: false,
        query: &[("page", false), ("size", false)],
    },
    BaseEndpointSpec {
        name: "create",
        method: "POST",
        path: "/",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "createAll",
        method: "POST",
        path: "/all",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "modify",
        method: "PUT",
        path: "/",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "modifyStatus",
        method: "PUT",
        path: "/status",
        body: true,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "remove",
        method: "DELETE",
        path: "/{id}",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "removeAll",
        method: "DELETE",
        path: "/",
        body: false,
        query: &[("ids", true)],
    },
    BaseEndpointSpec {
        name: "dict",
        method: "GET",
        path: "/dict",
        body: false,
        query: NO_QUERY,
    },
    BaseEndpointSpec {
        name: "dictField",
        method: "GET",
        path: "/dict/{fieldName}",
        body: false,
        query: NO_QUERY,
    },
];

pub fn scan(repo_path: &Path) -> Result<String, String> {
    let java_root = repo_path.join("src").join("main").join("java");
    if !java_root.is_dir() {
        return Err(format!(
            "未找到 {}，请确认这是 Java 项目的仓库根目录",
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
        let file_stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // 表单骨架只来自 *Form.java；DTO 等其余文件名直接跳过，避免读全仓
        if !file_stem.ends_with("Controller") && !file_stem.contains("Form") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                warnings.push(format!("读取 {} 失败: {error}", path.display()));
                continue;
            }
        };

        if file_stem.ends_with("Controller") {
            match parse_controller(&content) {
                Some(controller) => controllers.push(controller),
                None => warnings.push(format!("未能解析 {}", path.display())),
            }
        } else if let Some((name, form)) = parse_form(&content) {
            forms.insert(name, form);
        }
    }

    controllers.sort_by(|a, b| a.base_path.cmp(&b.base_path));
    let manifest = JavaManifest {
        framework: "java-scan".to_string(),
        controllers,
        forms,
        warnings,
    };

    serde_json::to_string_pretty(&manifest).map_err(|error| format!("序列化清单失败: {error}"))
}

// ============================================================
// 注释剥离与括号平衡扫描
// ============================================================

/// 剥除注释（保持行号，便于位置关联）。行注释排除 `://`（URL）。
fn strip_comments(content: &str) -> String {
    static RE_BLOCK: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"/\*[\s\S]*?\*/").unwrap());
    let block = &*RE_BLOCK;
    // `//` 前必须是行首或非冒号字符，避免误伤 "http://"
    static RE_LINE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?m)(^|[^:])//[^\n]*").unwrap());
    let line = &*RE_LINE;

    let blank = |text: &str| -> String {
        text.chars()
            .map(|ch| if ch == '\n' { '\n' } else { ' ' })
            .collect()
    };

    let without_block = block.replace_all(content, |caps: &regex::Captures| blank(&caps[0]));
    line.replace_all(&without_block, |caps: &regex::Captures| {
        let mut kept = caps[1].to_string();
        let comment_len = caps[0].len() - caps[1].len();
        kept.push_str(&" ".repeat(comment_len));
        kept
    })
    .into_owned()
}

/// 从 `open`（指向 `(`）扫描到平衡的 `)`，返回其位置（含）。
/// 跳过字符串/字符字面量中的括号。
fn scan_parenthesized(src: &str, open: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
            }
            b'\'' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'\'' {
                        break;
                    }
                    i += 1;
                }
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// 跳过空白
fn skip_ws(src: &str, mut i: usize) -> usize {
    let bytes = src.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// 解析从 `at`（指向 `@`）开始的注解：`@Name` + 可选平衡括号参数。
/// 返回 (注解名, 参数文本, 结束位置之后)。
fn parse_annotation(src: &str, at: usize) -> Option<(String, String, usize)> {
    let bytes = src.as_bytes();
    if bytes.get(at) != Some(&b'@') {
        return None;
    }
    let mut i = at + 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i == at + 1 {
        return None;
    }
    let name = src[at + 1..i].to_string();
    let next = skip_ws(src, i);
    if bytes.get(next) == Some(&b'(') {
        let close = scan_parenthesized(src, next)?;
        Some((name, src[next + 1..close].to_string(), close + 1))
    } else {
        Some((name, String::new(), i))
    }
}

// ============================================================
// Controller 解析
// ============================================================

fn parse_controller(raw: &str) -> Option<ControllerDef> {
    let content = strip_comments(raw);

    static RE_PACKAGE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?m)^package\s+([\w.]+);").unwrap());
    static RE_CLASS_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)public\s+(?:abstract\s+)?class\s+(\w+Controller)(?:\s+extends\s+(\w+))?(?:\s*<([^>]*)>)?").unwrap()
    });
    let package = &*RE_PACKAGE;
    let class_decl = &*RE_CLASS_DECL;

    let class = class_decl.captures(&content)?;
    let class_name = class.get(1)?.as_str().to_string();
    let super_name = class.get(2).map(|m| m.as_str().to_string());
    let generics = class.get(3).map(|m| m.as_str().to_string());

    let module = package
        .captures(&content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .and_then(|pkg| {
            let segments: Vec<&str> = pkg.split('.').collect();
            if segments.len() >= 2 && segments[segments.len() - 1] == "controller" {
                Some(segments[segments.len() - 2].to_string())
            } else {
                Some("common".to_string())
            }
        })
        .unwrap_or_else(|| "common".to_string());

    let base_path = extract_class_mapping(&content, class.get(0)?.start())
        .unwrap_or_else(|| format!("/{}", kebab(class_name.trim_end_matches("Controller"))));

    let (framework_kind, base_table, search_form, update_form) = match super_name.as_deref() {
        Some("AbstractBaseController") => {
            let (search, update) = split_form_generics(&generics);
            ("vef", Some(VEF_BASE_ENDPOINTS), search, update)
        }
        Some("AbstractFrameworkController") => {
            let (search, update) = split_form_generics(&generics);
            ("dhyx", Some(DHYX_BASE_ENDPOINTS), search, update)
        }
        _ => ("none", None, None, None),
    };

    let class_label = extract_class_api_label(&content, class.get(0)?.start());
    let declared = parse_declared_endpoints(&content, class.get(0)?.end());

    let mut endpoints = declared;
    if let Some(table) = base_table {
        for spec in table {
            let overridden = endpoints
                .iter()
                .any(|endpoint| endpoint.method == spec.method && endpoint.path == spec.path);
            if overridden {
                continue;
            }
            endpoints.push(EndpointDef {
                name: spec.name.to_string(),
                label: None,
                method: spec.method.to_string(),
                path: spec.path.to_string(),
                source: "base",
                body_form: if spec.body { update_form.clone() } else { None },
                query_params: spec
                    .query
                    .iter()
                    .map(|(name, required)| QueryParamDef {
                        name: name.to_string(),
                        required: *required,
                    })
                    .collect(),
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
        label: class_label,
        framework_kind,
        update_form,
        search_form,
        endpoints,
    })
}

/// 泛型参数列表第 3/4 位约定为 SearchForm/UpdateForm
fn split_form_generics(generics: &Option<String>) -> (Option<String>, Option<String>) {
    let types = generics
        .as_ref()
        .map(|raw| {
            raw.split(',')
                .map(|item| simple_type(item.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let search = types.get(2).cloned().filter(|name| name != "?");
    let update = types.get(3).cloned().filter(|name| name != "?");
    (search, update)
}

/// 类级 @RequestMapping：取 class 声明前最后一个 mapping 注解里的
/// 第一个字符串字面量（value 或 path 属性，剥掉 produces 等）
fn extract_class_mapping(content: &str, class_start: usize) -> Option<String> {
    let head = &content[..class_start];
    static RE_MAPPING: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"@RequestMapping\s*(?:\(([^;]*?)\))?"#).unwrap());
    let mapping = &*RE_MAPPING;
    let caps = mapping.captures(head)?;
    let args = caps
        .get(1)
        .map(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    static RE_LITERAL: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#""([^"]*)""#).unwrap());
    let literal = &*RE_LITERAL;
    literal
        .captures(&args)
        .map(|c| c.get(1).unwrap().as_str().to_string())
}

/// 类级 swagger @Api(tags = "...") 中文标签。
/// 注意与 @ApiOperation/@ApiParam 区分：要求注解名恰为 `Api`。
fn extract_class_api_label(content: &str, class_start: usize) -> Option<String> {
    let head = &content[..class_start];
    static RE_API: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@Api\s*\(").unwrap());
    let api = &*RE_API;
    let at_pos = api.find(head)?.start();
    let (_, args, _) = parse_annotation(head, at_pos)?;

    static RE_TAGS_ARRAY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"tags\s*=\s*\{\s*"([^"]*)""#).unwrap());
    let tags_array = &*RE_TAGS_ARRAY;
    if let Some(caps) = tags_array.captures(&args) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    static RE_LITERAL: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#""([^"]*)""#).unwrap());
    let literal = &*RE_LITERAL;
    literal
        .captures(&args)
        .map(|c| c.get(1).unwrap().as_str().to_string())
}

/// class 体里声明的方法级端点：括号平衡扫描，容忍 mapping 注解与
/// 签名之间夹带的任意注解（含 @ApiImplicitParams 嵌套大括号）。
fn parse_declared_endpoints(content: &str, body_start: usize) -> Vec<EndpointDef> {
    let body = &content[body_start..];
    static RE_MAPPING: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@(Get|Post|Put|Delete|Patch)Mapping").unwrap());
    let mapping = &*RE_MAPPING;

    static RE_REQUEST_PARAM_PAREN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"@RequestParam\(([^)]*)\)\s*(?:[\w.<>\[\]]+\s+)?([\w.<>\[\]]+)\s+(\w+)"#)
            .unwrap()
    });
    static RE_REQUEST_PARAM_BARE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@RequestParam\s+([\w.<>\[\]]+)\s+(\w+)").unwrap());
    static RE_REQUEST_BODY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@RequestBody\s+([\w.]+)").unwrap());
    static RE_LITERAL: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#""([^"]*)""#).unwrap());
    static RE_ATTR_NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"name\s*=\s*"([^"]*)""#).unwrap());
    static RE_ATTR_REQUIRED: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"required\s*=\s*(\w+)").unwrap());
    static RE_METHOD_NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"([A-Za-z_]\w*)\s*$").unwrap());
    let request_param_paren = &*RE_REQUEST_PARAM_PAREN;
    let request_param_bare = &*RE_REQUEST_PARAM_BARE;
    let request_body = &*RE_REQUEST_BODY;
    let literal = &*RE_LITERAL;
    let attr_name = &*RE_ATTR_NAME;
    let attr_required = &*RE_ATTR_REQUIRED;
    let method_name = &*RE_METHOD_NAME;

    // 预扫 @ApiOperation 位置，端点按「最近的、未被分隔」的注解取中文标签
    static RE_API_OPERATION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@ApiOperation\s*\(").unwrap());
    let api_operation = &*RE_API_OPERATION;
    let operation_positions: Vec<usize> = api_operation
        .find_iter(body)
        .map(|found| found.start())
        .collect();

    let mut endpoints = Vec::new();
    for found in mapping.captures_iter(body) {
        let mapping_start = found.get(0).unwrap().start();
        let http_method = found
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_ascii_uppercase();
        let (_, args, after_mapping) = match parse_annotation(body, mapping_start) {
            Some(hit) => hit,
            None => continue,
        };

        // 跳过 mapping 与签名之间的其它注解（任意嵌套）
        let mut cursor = skip_ws(body, after_mapping);
        while body.as_bytes().get(cursor) == Some(&b'@') {
            match parse_annotation(body, cursor) {
                Some((_, _, end)) => cursor = skip_ws(body, end),
                None => break,
            }
        }

        // 签名：到 `(` 为止，取最后一个标识符为方法名；参数到平衡 `)`
        let signature_head = &body[cursor..];
        static RE_VISIBILITY: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"(?:public|protected)\s").unwrap());
        let visibility = &*RE_VISIBILITY;
        if !visibility.is_match(signature_head) {
            continue;
        }
        let paren_offset = match signature_head.find('(') {
            Some(offset) => offset,
            None => continue,
        };
        // `(` 之前必须仍是本方法签名（不允许出现 `;`/`{`/`}`）
        let before_paren = &signature_head[..paren_offset];
        if before_paren.contains(';') || before_paren.contains('{') || before_paren.contains('}') {
            continue;
        }
        let Some(name_caps) = method_name.captures(before_paren) else {
            continue;
        };
        let name = name_caps.get(1).unwrap().as_str().to_string();

        let paren_pos = cursor + paren_offset;
        let Some(close_pos) = scan_parenthesized(body, paren_pos) else {
            continue;
        };
        let params_text = &body[paren_pos + 1..close_pos];

        let path = match literal.captures(&args) {
            Some(caps) => normalize_sub_path(caps.get(1).unwrap().as_str()),
            None => "/".to_string(),
        };

        // 中文标签：mapping 前最近的 @ApiOperation，且中间没有 `;` `}` 跨方法
        let label = operation_positions
            .iter()
            .rev()
            .find(|&&op_pos| op_pos < mapping_start)
            .filter(|&&op_pos| {
                !body[op_pos..mapping_start].contains(';')
                    && !body[op_pos..mapping_start].contains('}')
            })
            .and_then(|&op_pos| {
                let (_, op_args, _) = parse_annotation(body, op_pos)?;
                static RE_VALUE: std::sync::LazyLock<Regex> =
                    std::sync::LazyLock::new(|| Regex::new(r#"value\s*=\s*"([^"]*)""#).unwrap());
                let value = &*RE_VALUE;
                value
                    .captures(&op_args)
                    .map(|caps| caps.get(1).unwrap().as_str().to_string())
                    .or_else(|| {
                        literal
                            .captures(&op_args)
                            .map(|caps| caps.get(1).unwrap().as_str().to_string())
                    })
            });

        let body_form = request_body
            .captures(params_text)
            .map(|caps| simple_type(caps.get(1).unwrap().as_str()));

        let mut query_params = Vec::new();
        for param in request_param_paren.captures_iter(params_text) {
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
        for param in request_param_bare.captures_iter(params_text) {
            let variable = param.get(2).unwrap().as_str().to_string();
            query_params.push(QueryParamDef {
                name: variable,
                required: true,
            });
        }

        endpoints.push(EndpointDef {
            name,
            label,
            method: http_method,
            path,
            source: "declared",
            body_form,
            query_params,
        });
    }

    endpoints
}

fn normalize_sub_path(sub_path: &str) -> String {
    if sub_path.is_empty() {
        return "/".to_string();
    }
    if sub_path.starts_with('/') {
        sub_path.to_string()
    } else {
        format!("/{sub_path}")
    }
}

// ============================================================
// Form 解析（UpdateForm / SearchForm 字段表）
// ============================================================

fn parse_form(raw: &str) -> Option<(String, FormDef)> {
    let content = strip_comments(raw);

    static RE_CLASS_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)public\s+(?:abstract\s+)?class\s+(\w+)(?:\s+extends\s+([\w.]+))?(?:\s+implements\s+([\w.,\s]+?))?\s*\{").unwrap()
    });
    let class_decl = &*RE_CLASS_DECL;
    let caps = class_decl.captures(&content)?;
    let name = caps.get(1)?.as_str().to_string();
    let extends = caps
        .get(2)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    let implements = caps
        .get(3)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    // 抽象基类本身与无关类跳过；命中 Update/Search 表单约定（类或接口）才解析
    let is_form = |text: &str| {
        let lower = text.to_ascii_lowercase();
        lower.contains("updateform")
            || lower.contains("updatebaseform")
            || lower.contains("searchform")
            || lower.contains("searchbaseform")
    };
    if name.starts_with("Abstract") || name.starts_with("Base") {
        return None;
    }
    if !is_form(&extends) && !is_form(&implements) {
        return None;
    }

    static RE_COMMENT: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"@Comment\s*\(\s*"([^"]*)""#).unwrap());
    static RE_API_MODEL_PROP: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"@ApiModelProperty\s*\(([^)]*)\)"#).unwrap());
    static RE_PROP_VALUE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"value\s*=\s*"([^"]*)""#).unwrap());
    static RE_PROP_REQUIRED: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"required\s*=\s*true").unwrap());
    static RE_FIELD_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^\s*(?:public|protected|private)?\s*(?:static\s+)?(?:final\s+)?([\w.<>\[\], ]+?)\s+(\w+)\s*(?:=[^;]*)?;").unwrap()
    });
    let comment = &*RE_COMMENT;
    let api_model_prop = &*RE_API_MODEL_PROP;
    let prop_value = &*RE_PROP_VALUE;
    let prop_required = &*RE_PROP_REQUIRED;
    let field_decl = &*RE_FIELD_DECL;

    // 只解析类体第一层花括号内的字段（方法体内的局部变量跳过）
    let mut fields = Vec::new();
    let mut pending_comment: Option<String> = None;
    let mut pending_required = false;
    let mut depth: i32 = 0;
    let mut entered_class = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let opens = trimmed.matches('{').count() as i32;
        let closes = trimmed.matches('}').count() as i32;

        if entered_class && depth == 1 {
            if trimmed.starts_with('@') {
                if let Some(caps) = comment.captures(trimmed) {
                    pending_comment = Some(caps.get(1).unwrap().as_str().to_string());
                }
                if let Some(caps) = api_model_prop.captures(trimmed) {
                    let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                    if pending_comment.is_none() {
                        pending_comment = prop_value
                            .captures(attrs)
                            .map(|c| c.get(1).unwrap().as_str().to_string());
                    }
                    if prop_required.is_match(attrs) {
                        pending_required = true;
                    }
                }
                if trimmed.starts_with("@NotNull")
                    || trimmed.starts_with("@NotBlank")
                    || trimmed.starts_with("@NotEmpty")
                {
                    pending_required = true;
                }
            } else if let Some(caps) = field_decl.captures(line) {
                let field_type = caps.get(1).unwrap().as_str().trim().to_string();
                let field_name = caps.get(2).unwrap().as_str().to_string();
                if !field_type.is_empty()
                    && !matches!(field_type.as_str(), "return" | "new" | "throw")
                {
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

        if trimmed.contains("class ") && trimmed.contains('{') {
            entered_class = true;
        }
        depth += opens - closes;
    }

    if fields.is_empty() {
        return None;
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

    // ---------------- vef 风格（mrqcm） ----------------

    const VEF_CONTROLLER: &str = r#"
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

    const VEF_EXTERNAL: &str = r#"
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

    const VEF_UPDATE_FORM: &str = r#"
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

    const VEF_SEARCH_FORM: &str = r#"
package com.dhyx.mrqcm.system.form.search;

public class DepartmentSearchForm extends AbstractSearchForm {

    @SearchCondition(strategy = ConditionType.EQUALS)
    private SwitchStatus status;

    @SearchCondition(strategy = ConditionType.LIKE)
    @Comment("部门名称")
    private String name;
}
"#;

    // ---------------- dhyx 风格（peis） ----------------

    const DHYX_CONTROLLER: &str = r#"
package com.dhyx.server.system.controller;

@Api(tags = "机构管理控制器")
@ApiSort(1)
@RestController
@RequestMapping("/system/org")
@AllArgsConstructor
@Slf4j
public class OrgController extends AbstractFrameworkController<Org, OrgService, OrgSearchForm, OrgUpdateForm> {

    @ApiOperation(value = "获取机构树")
    @GetMapping(value = "/tree", produces = "application/json;charset=UTF-8")
    public Result<List<Tree<String>>> getOrgTree(@RequestParam(required = false, value = "parentId") String parentId) {
        return null;
    }

    @ApiImplicitParams(value = {@ApiImplicitParam(name = "id", value = "主键", required = true, dataType = "String", paramType = "path")})
    @ApiOperation(value = "按编码查询")
    @GetMapping(value = "/code/{code}")
    public Result<Org> getByCode(@PathVariable String code) {
        return null;
    }
}
"#;

    const DHYX_AUTH_CONTROLLER: &str = r#"
package com.dhyx.server.framework.auth.controller;

// public class FakeControllerInComment extends AbstractFrameworkController<A, B, C, D> {

@Api(tags = "认证控制器")
@RestController
@RequestMapping("/auth/oauth")
public class AuthController {

    @ApiOperation(value = "获取访问令牌")
    @PostMapping(value = "/token", produces = "application/json;charset=UTF-8")
    public Result<Map<String, Object>> token(@RequestParam Map<String, String> params) {
        return null;
    }
}
"#;

    const DHYX_UPDATE_FORM: &str = r#"
package com.dhyx.server.system.form.update;

public class OrgUpdateForm extends AbstractUpdateBaseForm {
    /**
     * 父编号
     */
    @ApiModelProperty(value = "父编号", allowableValues = "", required = true, position = 1)
    @NotNull(message = "父编号{required}")
    private String parentId;

    @ApiModelProperty(value = "类别")
    @NotBlank
    private String type;

    private String remark;
}
"#;

    const DHYX_SEARCH_FORM: &str = r#"
package com.dhyx.server.system.form.search;

public class OrgSearchForm implements SearchBaseForm {

    @ApiModelProperty(value = "名字")
    @SearchCondition(strategy = ConditionEnum.LIKE)
    String name;

    @ApiModelProperty(value = "编码")
    private String code;

    private Boolean onlySelectChildOrg;
}
"#;

    #[test]
    fn parses_vef_controller_with_base_inheritance() {
        let controller = parse_controller(VEF_CONTROLLER).expect("controller");

        assert_eq!(controller.module, "system");
        assert_eq!(controller.base_path, "/system/department");
        assert_eq!(controller.framework_kind, "vef");
        assert!(controller.label.is_none());
        assert_eq!(
            controller.search_form.as_deref(),
            Some("DepartmentSearchForm")
        );
        assert_eq!(
            controller.update_form.as_deref(),
            Some("DepartmentUpdateForm")
        );
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

        let create = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "create")
            .expect("base create");
        assert_eq!(create.body_form.as_deref(), Some("DepartmentUpdateForm"));
    }

    #[test]
    fn parses_vef_standalone_controller() {
        let controller = parse_controller(VEF_EXTERNAL).expect("controller");

        assert_eq!(controller.module, "api");
        assert_eq!(controller.framework_kind, "none");
        assert_eq!(controller.endpoints.len(), 2);

        let file = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/medical-record-file")
            .expect("medical-record-file");
        assert_eq!(file.query_params[0].name, "patientId");
        assert!(file.query_params[0].required);
    }

    #[test]
    fn parses_dhyx_controller_with_swagger_labels_and_nested_annotations() {
        let controller = parse_controller(DHYX_CONTROLLER).expect("controller");

        assert_eq!(controller.module, "system");
        assert_eq!(controller.base_path, "/system/org");
        assert_eq!(controller.framework_kind, "dhyx");
        assert_eq!(controller.label.as_deref(), Some("机构管理控制器"));
        assert_eq!(controller.search_form.as_deref(), Some("OrgSearchForm"));
        assert_eq!(controller.update_form.as_deref(), Some("OrgUpdateForm"));

        // 2 个声明端点 + 10 个基类端点
        assert_eq!(controller.endpoints.len(), 12);

        let tree = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/tree")
            .expect("tree endpoint");
        assert_eq!(tree.label.as_deref(), Some("获取机构树"));
        assert_eq!(tree.query_params[0].name, "parentId");
        assert!(!tree.query_params[0].required);

        // @ApiImplicitParams 嵌套注解夹在中间仍能解析
        let by_code = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.path == "/code/{code}")
            .expect("code endpoint with nested annotation");
        assert_eq!(by_code.name, "getByCode");
        assert_eq!(by_code.label.as_deref(), Some("按编码查询"));

        // 基类端点展开
        let page = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "page")
            .expect("base page");
        assert_eq!((page.method.as_str(), page.path.as_str()), ("GET", "/"));
        assert_eq!(
            page.query_params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["page", "size"]
        );

        let get = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "get")
            .expect("base get");
        assert_eq!(get.path, "/{id}");
        assert!(get.query_params.is_empty(), "路径参数不进 query");

        let create_all = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "createAll")
            .expect("base createAll");
        assert_eq!(create_all.body_form.as_deref(), Some("OrgUpdateForm"));

        let dict_field = controller
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == "dictField")
            .expect("base dictField");
        assert_eq!(dict_field.path, "/dict/{fieldName}");
    }

    #[test]
    fn ignores_commented_out_class_declarations() {
        let controller = parse_controller(DHYX_AUTH_CONTROLLER).expect("controller");

        // 注释里的 FakeControllerInComment 必须被忽略
        assert_eq!(controller.name, "AuthController");
        assert_eq!(controller.framework_kind, "none");
        assert_eq!(controller.label.as_deref(), Some("认证控制器"));
        assert_eq!(controller.endpoints.len(), 1);
        assert_eq!(
            controller.endpoints[0].label.as_deref(),
            Some("获取访问令牌")
        );
    }

    #[test]
    fn parses_vef_and_dhyx_forms() {
        let (name, form) = parse_form(VEF_UPDATE_FORM).expect("vef update form");
        assert_eq!(name, "DepartmentUpdateForm");
        assert_eq!(form.fields.len(), 3);
        assert_eq!(form.fields[0].label.as_deref(), Some("部门名称"));
        assert!(form.fields[0].required);
        assert_eq!(form.fields[1].field_type, "DepartmentType");
        assert!(!form.fields[2].required);

        let (name, form) = parse_form(VEF_SEARCH_FORM).expect("vef search form");
        assert_eq!(name, "DepartmentSearchForm");
        assert_eq!(form.fields[1].label.as_deref(), Some("部门名称"));

        // dhyx：extends AbstractUpdateBaseForm + @ApiModelProperty
        let (name, form) = parse_form(DHYX_UPDATE_FORM).expect("dhyx update form");
        assert_eq!(name, "OrgUpdateForm");
        assert_eq!(form.fields.len(), 3);
        assert_eq!(form.fields[0].label.as_deref(), Some("父编号"));
        assert!(
            form.fields[0].required,
            "ApiModelProperty required=true 或 @NotNull 均视为必填"
        );
        assert!(form.fields[1].required);
        assert!(!form.fields[2].required);

        // dhyx：implements SearchBaseForm + 包级私有字段
        let (name, form) = parse_form(DHYX_SEARCH_FORM).expect("dhyx search form");
        assert_eq!(name, "OrgSearchForm");
        assert_eq!(form.fields.len(), 3);
        assert_eq!(form.fields[0].name, "name");
        assert_eq!(form.fields[0].field_type, "String");
        assert_eq!(form.fields[0].label.as_deref(), Some("名字"));
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
    fn scan_real_repos_if_present() {
        // 开发机上有真实仓库时做端到端冒烟；其他机器跳过
        for (repo, min_controllers, min_endpoints) in [
            (
                "/Users/jujiuyey/dhyxtech-work/mrqcm/mrqcm-server",
                100usize,
                1500usize,
            ),
            (
                "/Users/jujiuyey/dhyxtech-work/peis/peis-server",
                100usize,
                1200usize,
            ),
        ] {
            let repo = Path::new(repo);
            if !repo.is_dir() {
                continue;
            }
            let json = scan(repo).expect("scan real repo");
            let manifest: serde_json::Value = serde_json::from_str(&json).expect("valid json");
            let controllers = manifest["controllers"].as_array().expect("controllers");
            assert!(
                controllers.len() > min_controllers,
                "got {}",
                controllers.len()
            );
            let total: usize = controllers
                .iter()
                .map(|c| c["endpoints"].as_array().map(Vec::len).unwrap_or(0))
                .sum();
            assert!(total > min_endpoints, "got {total}");
        }
    }
}

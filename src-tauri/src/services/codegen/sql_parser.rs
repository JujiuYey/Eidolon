use regex::Regex;

use super::types::{ParsedField, ParsedTable};

const FILTERED_AUDIT_FIELDS: &[&str] = &[
    "id",
    "created_at",
    "created_by",
    "created_by_name",
    "updated_at",
    "updated_by",
    "updated_by_name",
    "deleted_at",
    "deleted_by",
    "tenant_id",
];

/// SQL statement types for multi-statement support
#[derive(Debug, Clone)]
enum SqlStatement {
    CreateTable(String),
    CommentOnTable { table: String, comment: String },
    CommentOnColumn { table: String, column: String, comment: String },
}

/// Split SQL into individual statements, handling quotes and parentheses
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut previous_char = '\0';

    for ch in sql.chars() {
        // Handle escaped quotes
        if in_single_quote {
            current.push(ch);
            if ch == '\'' && previous_char != '\\' {
                in_single_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_double_quote {
            current.push(ch);
            if ch == '"' && previous_char != '\\' {
                in_double_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_backtick {
            current.push(ch);
            if ch == '`' {
                in_backtick = false;
            }
            previous_char = ch;
            continue;
        }

        match ch {
            '\'' => {
                in_single_quote = true;
                current.push(ch);
            }
            '"' => {
                in_double_quote = true;
                current.push(ch);
            }
            '`' => {
                in_backtick = true;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    statements.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }

        previous_char = ch;
    }

    // Don't forget the last statement
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(trimmed.to_string());
    }

    statements
}

/// Parse a COMMENT ON statement
fn parse_comment_statement(stmt: &str) -> Option<SqlStatement> {
    let trimmed = stmt.trim();
    
    // Try COMMENT ON TABLE
    let table_regex = Regex::new(r#"(?i)^\s*COMMENT\s+ON\s+TABLE\s+`?([a-zA-Z0-9_.]+)`?\s+(?:IS|=)\s*'([^']*)'\s*;?\s*$"#).ok()?;
    if let Some(captures) = table_regex.captures(trimmed) {
        let table = captures.get(1)?.as_str().to_string();
        let comment = captures.get(2)?.as_str().to_string();
        return Some(SqlStatement::CommentOnTable { table, comment });
    }

    // Try COMMENT ON COLUMN
    let column_regex = Regex::new(r#"(?i)^\s*COMMENT\s+ON\s+COLUMN\s+`?([a-zA-Z0-9_.]+)`?\s*\.\s*`?([a-zA-Z0-9_]+)`?\s+(?:IS|=)\s*'([^']*)'\s*;?\s*$"#).ok()?;
    if let Some(captures) = column_regex.captures(trimmed) {
        let table = captures.get(1)?.as_str().to_string();
        let column = captures.get(2)?.as_str().to_string();
        let comment = captures.get(3)?.as_str().to_string();
        return Some(SqlStatement::CommentOnColumn { table, column, comment });
    }

    None
}

/// Classify a SQL statement
fn classify_statement(stmt: &str) -> Option<SqlStatement> {
    let trimmed = stmt.trim();
    
    // Check if it's a CREATE TABLE
    let create_table_regex = Regex::new(r#"(?i)^\s*CREATE\s+TABLE"#).ok()?;
    if create_table_regex.is_match(trimmed) {
        return Some(SqlStatement::CreateTable(trimmed.to_string()));
    }

    // Try COMMENT ON statements
    parse_comment_statement(trimmed)
}

pub fn parse_sql_ddl(sql: &str) -> Result<ParsedTable, String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("SQL 不能为空".to_string());
    }

    // Split into statements and classify
    let statements = split_sql_statements(trimmed);
    if statements.is_empty() {
        return Err("未找到有效的 SQL 语句".to_string());
    }

    // Classify all statements
    let mut classified = Vec::new();
    for stmt in &statements {
        if let Some(classified_stmt) = classify_statement(stmt) {
            classified.push(classified_stmt);
        }
    }

    // Find the CREATE TABLE statement
    let create_table_stmt = classified
        .iter()
        .find_map(|s| match s {
            SqlStatement::CreateTable(sql) => Some(sql.clone()),
            _ => None,
        })
        .ok_or_else(|| "未找到 CREATE TABLE 语句".to_string())?;

    // Parse the CREATE TABLE (existing logic)
    let (table_name, body, tail) = extract_create_table_sections(&create_table_stmt)?;
    
    // Extract table comment from CREATE TABLE tail
    let mut entity_label = extract_table_comment(&tail);
    
    // Process COMMENT ON statements for additional comments
    let mut column_comments: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for stmt in &classified {
        match stmt {
            SqlStatement::CommentOnTable { table, comment } => {
                // Only use if we don't already have a table comment
                if entity_label.is_none() {
                    entity_label = Some(comment.clone());
                }
            }
            SqlStatement::CommentOnColumn { table, column, comment } => {
                // Store column comments by column name
                let key = format!("{}.{}", table, column);
                column_comments.insert(key, comment.clone());
            }
            _ => {}
        }
    }

    // Parse fields (existing logic)
    let mut fields = Vec::new();

    for segment in split_top_level_segments(&body) {
        if let Some(mut field) = parse_column_definition(&segment)? {
            // Check if this field has a COMMENT ON annotation
            let comment_key = format!("{}.{}", table_name, field.name);
            if let Some(comment) = column_comments.get(&comment_key) {
                // Only update if the inline comment was empty or not set
                if field.comment.is_none() {
                    field.comment = Some(comment.clone());
                }
            }
            
            if FILTERED_AUDIT_FIELDS.contains(&field.name.as_str()) {
                continue;
            }
            fields.push(field);
        }
    }

    if fields.is_empty() {
        return Err("所有字段均为审计字段，请检查 SQL".to_string());
    }

    Ok(ParsedTable {
        table_name: table_name.clone(),
        entity_name: derive_entity_name(&table_name),
        entity_label,
        fields,
    })
}

fn extract_create_table_sections(sql: &str) -> Result<(String, String, String), String> {
    let regex =
        Regex::new(r#"(?is)create\s+table\s+(?:if\s+not\s+exists\s+)?`?([a-zA-Z0-9_]+)`?\s*\("#)
            .map_err(|error| format!("构建建表语句匹配失败: {error}"))?;
    let captures = regex
        .captures(sql)
        .ok_or_else(|| "未找到有效的 CREATE TABLE 语句".to_string())?;
    let table_name = captures
        .get(1)
        .map(|capture| capture.as_str().to_string())
        .ok_or_else(|| "无法读取表名".to_string())?;
    let full_match = captures
        .get(0)
        .ok_or_else(|| "无法定位建表语句".to_string())?;
    let open_paren_index = full_match.end() - 1;
    let close_paren_index = find_matching_paren(sql, open_paren_index)
        .ok_or_else(|| "未找到 CREATE TABLE 的结束括号".to_string())?;

    Ok((
        table_name,
        sql[open_paren_index + 1..close_paren_index].to_string(),
        sql[close_paren_index + 1..].to_string(),
    ))
}

fn find_matching_paren(source: &str, open_paren_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut previous_char = '\0';

    for (index, ch) in source
        .char_indices()
        .skip_while(|(index, _)| *index < open_paren_index)
    {
        if in_single_quote {
            if ch == '\'' && previous_char != '\\' {
                in_single_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_double_quote {
            if ch == '"' && previous_char != '\\' {
                in_double_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_backtick {
            if ch == '`' {
                in_backtick = false;
            }
            previous_char = ch;
            continue;
        }

        match ch {
            '\'' => in_single_quote = true,
            '"' => in_double_quote = true,
            '`' => in_backtick = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }

        previous_char = ch;
    }

    None
}

fn split_top_level_segments(body: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut previous_char = '\0';

    for ch in body.chars() {
        if in_single_quote {
            current.push(ch);
            if ch == '\'' && previous_char != '\\' {
                in_single_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_double_quote {
            current.push(ch);
            if ch == '"' && previous_char != '\\' {
                in_double_quote = false;
            }
            previous_char = ch;
            continue;
        }

        if in_backtick {
            current.push(ch);
            if ch == '`' {
                in_backtick = false;
            }
            previous_char = ch;
            continue;
        }

        match ch {
            '\'' => {
                in_single_quote = true;
                current.push(ch);
            }
            '"' => {
                in_double_quote = true;
                current.push(ch);
            }
            '`' => {
                in_backtick = true;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    segments.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }

        previous_char = ch;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }

    segments
}

fn parse_column_definition(segment: &str) -> Result<Option<ParsedField>, String> {
    let trimmed = segment.trim().trim_end_matches(',').trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let uppercase = trimmed.to_ascii_uppercase();
    let constraint_prefixes = [
        "PRIMARY KEY",
        "UNIQUE KEY",
        "UNIQUE INDEX",
        "UNIQUE",
        "KEY",
        "INDEX",
        "CONSTRAINT",
        "FOREIGN KEY",
    ];
    if constraint_prefixes
        .iter()
        .any(|prefix| uppercase.starts_with(prefix))
    {
        return Ok(None);
    }

    let (name, rest) = split_identifier_prefix(trimmed)?;
    let db_type = extract_db_type(rest)?;
    let nullable = !uppercase.contains("NOT NULL") && !uppercase.contains("PRIMARY KEY");
    let is_primary_key = uppercase.contains("PRIMARY KEY");
    let default_value = extract_default_value(rest);
    let comment = extract_comment(rest);
    let go_type = map_sql_type_to_go(&db_type, nullable && default_value.is_none())?;
    let validate = build_validate(&name, &db_type, &go_type, nullable, default_value.as_ref());

    Ok(Some(ParsedField {
        go_name: to_go_name(&name),
        json_name: to_json_name(&name),
        name,
        go_type,
        db_type,
        nullable,
        is_primary_key,
        default_value,
        comment,
        validate,
    }))
}

fn split_identifier_prefix(value: &str) -> Result<(String, &str), String> {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix('`') {
        let end = rest
            .find('`')
            .ok_or_else(|| format!("列定义缺少结束反引号: {trimmed}"))?;
        let name = rest[..end].to_string();
        let remainder = rest[end + 1..].trim_start();
        return Ok((name, remainder));
    }

    let split_index = trimmed
        .find(char::is_whitespace)
        .ok_or_else(|| format!("无法解析列定义: {trimmed}"))?;
    Ok((
        trimmed[..split_index].to_string(),
        trimmed[split_index..].trim_start(),
    ))
}

fn extract_db_type(rest: &str) -> Result<String, String> {
    let regex = Regex::new(r#"(?i)^([a-z]+(?:\([^)]*\))?)"#)
        .map_err(|error| format!("构建类型匹配失败: {error}"))?;
    let captures = regex
        .captures(rest)
        .ok_or_else(|| format!("无法解析字段类型: {rest}"))?;
    captures
        .get(1)
        .map(|capture| capture.as_str().to_ascii_uppercase())
        .ok_or_else(|| format!("无法读取字段类型: {rest}"))
}

fn extract_comment(rest: &str) -> Option<String> {
    let regex = Regex::new(r#"(?i)\bcomment\s*(?:=\s*)?'([^']*)'"#).ok()?;
    regex
        .captures(rest)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_table_comment(tail: &str) -> Option<String> {
    extract_comment(tail)
}

fn extract_default_value(rest: &str) -> Option<String> {
    let regex = Regex::new(r#"(?i)\bdefault\s+('([^']*)'|[^\s,]+)"#).ok()?;
    regex
        .captures(rest)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().trim_matches('\'').to_string())
}

fn map_sql_type_to_go(db_type: &str, nullable: bool) -> Result<String, String> {
    let normalized = db_type.to_ascii_uppercase();
    let mapped = if normalized.starts_with("VARCHAR")
        || normalized.starts_with("CHAR")
        || normalized.starts_with("TEXT")
        || normalized.starts_with("MEDIUMTEXT")
        || normalized.starts_with("LONGTEXT")
    {
        if nullable {
            "*string"
        } else {
            "string"
        }
    } else if normalized.starts_with("BIGINT") {
        if nullable {
            "*int64"
        } else {
            "int64"
        }
    } else if normalized.starts_with("SMALLINT")
        || normalized.starts_with("INT")
        || normalized.starts_with("INTEGER")
    {
        if nullable {
            "*int"
        } else {
            "int"
        }
    } else if normalized.starts_with("TINYINT(1)")
        || normalized.starts_with("BOOL")
        || normalized.starts_with("BOOLEAN")
    {
        if nullable {
            "*bool"
        } else {
            "bool"
        }
    } else if normalized.starts_with("TINYINT") {
        if nullable {
            "*int"
        } else {
            "int"
        }
    } else if normalized.starts_with("DECIMAL")
        || normalized.starts_with("FLOAT")
        || normalized.starts_with("DOUBLE")
    {
        if nullable {
            "*float64"
        } else {
            "float64"
        }
    } else if normalized.starts_with("DATETIME")
        || normalized.starts_with("TIMESTAMP")
        || normalized.starts_with("DATE")
        || normalized.starts_with("TIME")
    {
        if nullable {
            "*time.Time"
        } else {
            "time.Time"
        }
    } else if normalized.starts_with("JSON") {
        "map[string]any"
    } else if normalized.starts_with("BLOB") {
        "[]byte"
    } else {
        return Err(format!("暂不支持的 SQL 类型: {db_type}"));
    };

    Ok(mapped.to_string())
}

fn build_validate(
    name: &str,
    db_type: &str,
    go_type: &str,
    nullable: bool,
    default_value: Option<&String>,
) -> Option<String> {
    let required = !nullable && default_value.is_none();

    if name == "sort_order" && matches!(go_type, "int" | "*int") {
        return Some(if required {
            "required,min=0".to_string()
        } else {
            "omitempty,min=0".to_string()
        });
    }

    if matches!(go_type, "string" | "*string") {
        let mut rules = Vec::new();
        if required {
            rules.push("required".to_string());
        } else if extract_length(db_type).is_some() || is_identifier_like(name) {
            rules.push("omitempty".to_string());
        }

        if is_identifier_like(name) {
            rules.push("alphanum".to_string());
        }

        if let Some(length) = extract_length(db_type) {
            rules.push(format!("max={length}"));
        }

        if rules.is_empty() {
            return None;
        }

        return Some(rules.join(","));
    }

    None
}

fn extract_length(db_type: &str) -> Option<usize> {
    let regex = Regex::new(r#"\((\d+)"#).ok()?;
    regex
        .captures(db_type)
        .and_then(|captures| captures.get(1))
        .and_then(|capture| capture.as_str().parse::<usize>().ok())
}

fn is_identifier_like(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}

fn derive_entity_name(table_name: &str) -> String {
    let segments = table_name
        .split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let source = if segments.len() > 1 {
        segments[1..].join("_")
    } else {
        table_name.to_string()
    };

    source
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(to_go_segment)
        .collect::<String>()
}

fn to_go_name(value: &str) -> String {
    value
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(to_go_segment)
        .collect::<String>()
}

fn to_go_segment(segment: &str) -> String {
    match segment.to_ascii_lowercase().as_str() {
        "id" => "ID".to_string(),
        "api" => "API".to_string(),
        "url" => "URL".to_string(),
        "ip" => "IP".to_string(),
        "ui" => "UI".to_string(),
        "db" => "DB".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => {
                    let mut value = first.to_uppercase().to_string();
                    value.push_str(chars.as_str());
                    value
                }
                None => String::new(),
            }
        }
    }
}

fn to_json_name(value: &str) -> String {
    let segments = value
        .split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut result = String::new();

    for (index, segment) in segments.iter().enumerate() {
        let normalized = segment.to_ascii_lowercase();
        if index == 0 {
            result.push_str(&normalized);
            continue;
        }

        let mut chars = normalized.chars();
        if let Some(first) = chars.next() {
            result.push_str(&first.to_uppercase().to_string());
            result.push_str(chars.as_str());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::parse_sql_ddl;

    #[test]
    fn test_parse_sql_ddl_extracts_fields_comments_and_filters_audit_columns() {
        let sql = r#"
CREATE TABLE sys_user (
    id VARCHAR(32) PRIMARY KEY COMMENT '用户主键',
    organization_id VARCHAR(32) NOT NULL COMMENT '机构主键',
    name VARCHAR(64) NOT NULL COMMENT '用户名称',
    email VARCHAR(128) COMMENT '邮箱地址',
    is_active TINYINT(1) DEFAULT 1 COMMENT '是否启用',
    balance DECIMAL(10,2) COMMENT '账户余额',
    meta JSON COMMENT '元数据',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '更新时间'
) COMMENT='用户表';
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.table_name, "sys_user");
        assert_eq!(table.entity_name, "User");
        assert_eq!(table.entity_label.as_deref(), Some("用户表"));
        assert_eq!(table.fields.len(), 6);

        let organization_id = table
            .fields
            .iter()
            .find(|field| field.name == "organization_id")
            .expect("organization_id should exist");
        assert_eq!(organization_id.go_name, "OrganizationID");
        assert_eq!(organization_id.json_name, "organizationId");
        assert_eq!(organization_id.go_type, "string");
        assert_eq!(
            organization_id.validate.as_deref(),
            Some("required,alphanum,max=32")
        );

        let email = table
            .fields
            .iter()
            .find(|field| field.name == "email")
            .expect("email should exist");
        assert_eq!(email.go_type, "*string");
        assert_eq!(email.comment.as_deref(), Some("邮箱地址"));
        assert_eq!(email.validate.as_deref(), Some("omitempty,max=128"));

        let is_active = table
            .fields
            .iter()
            .find(|field| field.name == "is_active")
            .expect("is_active should exist");
        assert_eq!(is_active.go_type, "bool");

        let meta = table
            .fields
            .iter()
            .find(|field| field.name == "meta")
            .expect("meta should exist");
        assert_eq!(meta.go_type, "map[string]any");
        assert!(meta.validate.is_none());

        let balance = table
            .fields
            .iter()
            .find(|field| field.name == "balance")
            .expect("balance should exist");
        assert_eq!(balance.go_type, "*float64");

        assert!(!table.fields.iter().any(|field| field.name == "id"));
        assert!(!table.fields.iter().any(|field| field.name == "created_at"));
        assert!(!table.fields.iter().any(|field| field.name == "updated_at"));
    }

    #[test]
    fn test_parse_sql_ddl_supports_nested_module_style_entities() {
        let sql = r#"
CREATE TABLE hr_probation_config (
    sort_order INT NOT NULL COMMENT '排序',
    staff_limit BIGINT COMMENT '人数上限',
    started_at DATETIME COMMENT '生效时间'
);
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.entity_name, "ProbationConfig");
        assert_eq!(table.fields.len(), 3);

        let sort_order = table
            .fields
            .iter()
            .find(|field| field.name == "sort_order")
            .expect("sort_order should exist");
        assert_eq!(sort_order.go_type, "int");
        assert_eq!(sort_order.validate.as_deref(), Some("required,min=0"));

        let staff_limit = table
            .fields
            .iter()
            .find(|field| field.name == "staff_limit")
            .expect("staff_limit should exist");
        assert_eq!(staff_limit.go_type, "*int64");

        let started_at = table
            .fields
            .iter()
            .find(|field| field.name == "started_at")
            .expect("started_at should exist");
        assert_eq!(started_at.go_type, "*time.Time");
    }

    #[test]
    fn test_parse_sql_ddl_with_comment_on_statements() {
        // Test with separate COMMENT ON statements
        // Note: created_at will be filtered out as it's in FILTERED_AUDIT_FIELDS
        let sql = r#"
CREATE TABLE hr_contract_template_version (
    id VARCHAR(32) PRIMARY KEY,
    title VARCHAR(128) NOT NULL,
    content TEXT,
    created_at TIMESTAMP
);

COMMENT ON TABLE hr_contract_template_version IS '合同模板版本';
COMMENT ON COLUMN hr_contract_template_version.title IS '模板标题';
COMMENT ON COLUMN hr_contract_template_version.content IS '模板内容';
COMMENT ON COLUMN hr_contract_template_version.created_at IS '创建时间';
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.table_name, "hr_contract_template_version");
        assert_eq!(table.entity_name, "ContractTemplateVersion");
        assert_eq!(table.entity_label.as_deref(), Some("合同模板版本"));
        // created_at is filtered out as it's in FILTERED_AUDIT_FIELDS
        assert_eq!(table.fields.len(), 2);

        let title = table
            .fields
            .iter()
            .find(|field| field.name == "title")
            .expect("title should exist");
        assert_eq!(title.comment.as_deref(), Some("模板标题"));

        let content = table
            .fields
            .iter()
            .find(|field| field.name == "content")
            .expect("content should exist");
        assert_eq!(content.comment.as_deref(), Some("模板内容"));
    }

    #[test]
    fn test_parse_sql_ddl_with_mixed_inline_and_comment_on() {
        // Test with both inline comments and COMMENT ON statements
        // COMMENT ON should only update if inline comment is None
        let sql = r#"
CREATE TABLE hr_contract (
    id VARCHAR(32) PRIMARY KEY,
    title VARCHAR(128) NOT NULL COMMENT '合同标题',
    status INT COMMENT '合同状态'
);

COMMENT ON COLUMN hr_contract.status IS '合同状态(1=草稿,2=生效,3=终止)';
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.table_name, "hr_contract");
        assert_eq!(table.entity_name, "Contract");
        assert_eq!(table.fields.len(), 2);

        let title = table
            .fields
            .iter()
            .find(|field| field.name == "title")
            .expect("title should exist");
        // Should use inline comment (takes precedence - COMMENT ON doesn't override)
        assert_eq!(title.comment.as_deref(), Some("合同标题"));

        let status = table
            .fields
            .iter()
            .find(|field| field.name == "status")
            .expect("status should exist");
        // Should use inline comment (takes precedence - COMMENT ON doesn't override)
        assert_eq!(status.comment.as_deref(), Some("合同状态"));
    }

    #[test]
    fn test_parse_sql_ddl_comment_on_with_equals() {
        // Test COMMENT ON with = instead of IS
        let sql = r#"
CREATE TABLE test_table (
    id VARCHAR(32) PRIMARY KEY,
    name VARCHAR(64)
);

COMMENT ON TABLE test_table = '测试表';
COMMENT ON COLUMN test_table.name = '名称';
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.entity_label.as_deref(), Some("测试表"));
        
        let name = table
            .fields
            .iter()
            .find(|field| field.name == "name")
            .expect("name should exist");
        assert_eq!(name.comment.as_deref(), Some("名称"));
    }

    #[test]
    fn test_parse_sql_ddl_comment_on_only() {
        // Test COMMENT ON statements only (no inline comments)
        let sql = r#"
CREATE TABLE demo (
    id VARCHAR(32) PRIMARY KEY,
    title VARCHAR(64)
);

COMMENT ON TABLE demo IS '演示表';
COMMENT ON COLUMN demo.title IS '标题';
"#;

        let table = parse_sql_ddl(sql).expect("sql should parse");

        assert_eq!(table.entity_label.as_deref(), Some("演示表"));
        
        let title = table
            .fields
            .iter()
            .find(|field| field.name == "title")
            .expect("title should exist");
        assert_eq!(title.comment.as_deref(), Some("标题"));
    }
}

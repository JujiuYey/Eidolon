//! 导入解析：DDL 文本 / 智能查询结果 → ParsedSchema。
//!
//! - `parse_ddl`：按方言解析 CREATE TABLE / COMMENT ON / CREATE INDEX；
//!   逐语句切分解析，单条失败跳过并计数（容忍 mysqldump 的 SET/LOCK 等噪声）。
//! - `parse_query_result`：接受智能查询粘贴的 JSON 数组或带表头 TSV。

use std::collections::HashMap;

use sqlparser::ast::{ColumnOption, ObjectName, Statement, TableConstraint};
use sqlparser::dialect::{MySqlDialect, OracleDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::models::schema_dict::{ParsedColumn, ParsedIndex, ParsedSchema, ParsedTable};

/// DDL 解析结果：结构 + 跳过的语句数
pub struct DdlParseOutcome {
    pub schema: ParsedSchema,
    pub skipped_statements: usize,
}

/// 解析建表 DDL。db_type: mysql | pg | oracle
pub fn parse_ddl(text: &str, db_type: &str) -> Result<DdlParseOutcome, String> {
    let cleaned = preclean(text);
    if cleaned.trim().is_empty() {
        return Err("没有可解析的内容".to_string());
    }

    let dialect: Box<dyn sqlparser::dialect::Dialect> = match db_type {
        "pg" => Box::new(PostgreSqlDialect {}),
        "oracle" => Box::new(OracleDialect {}),
        _ => Box::new(MySqlDialect {}),
    };

    let tokens = Tokenizer::new(&*dialect, &cleaned)
        .tokenize()
        .map_err(|error| format!("SQL 分词失败: {error}"))?;

    // 按分号切成语句块；用 Token 的 Display 还原 SQL（引号等由 Display 保证）
    let mut chunks: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for token in &tokens {
        if matches!(token, Token::SemiColon) {
            chunks.push(current.join(" "));
            current.clear();
        } else if !matches!(token, Token::Whitespace(_)) {
            current.push(token.to_string());
        }
    }
    chunks.push(current.join(" "));

    let mut tables: Vec<ParsedTable> = Vec::new();
    let mut table_index: HashMap<String, usize> = HashMap::new();
    let mut table_comments: HashMap<String, String> = HashMap::new();
    let mut column_comments: HashMap<(String, String), String> = HashMap::new();
    let mut skipped = 0usize;

    for chunk in &chunks {
        if !chunk.contains(|character: char| !character.is_whitespace()) {
            continue;
        }
        let statements = match Parser::new(&*dialect).try_with_sql(chunk) {
            Ok(mut parser) => match parser.parse_statements() {
                Ok(statements) => statements,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            },
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        for statement in statements {
            collect_statement(
                statement,
                &mut tables,
                &mut table_index,
                &mut table_comments,
                &mut column_comments,
            );
        }
    }

    apply_comments(&mut tables, &table_comments, &column_comments);

    if tables.is_empty() {
        return Err(format!(
            "没有解析出任何表结构（跳过了 {skipped} 条无法识别的语句），请确认粘贴的是建表 DDL 或选择了正确的方言"
        ));
    }

    Ok(DdlParseOutcome {
        schema: ParsedSchema {
            tables,
            skipped_statements: skipped as i64,
        },
        skipped_statements: skipped,
    })
}

/// 清掉 mysqldump / Oracle dump 等来源的解析噪声：
/// `/*!NNNNN ...*/` 条件注释、DELIMITER 行、约束后的 ENABLE 关键字
fn preclean(text: &str) -> String {
    let without_directives = regex::Regex::new(r"(?s)/\*!.*?\*/")
        .map(|expression| expression.replace_all(text, "").to_string())
        .unwrap_or_else(|_| text.to_string());

    // Oracle dump 在 NOT NULL / NULL 后必带 ENABLE，sqlparser 不认
    let null_enable = regex::Regex::new(r"(?i)((?:NOT\s+)?NULL)\s+ENABLE\b")
        .map(|expression| expression.replace_all(&without_directives, "$1").to_string())
        .unwrap_or_else(|_| without_directives);
    // 表级约束结尾的 ENABLE（如 `PRIMARY KEY ("ID") ENABLE`）
    let constraint_enable = regex::Regex::new(r"(?i)\)\s+ENABLE\b")
        .map(|expression| expression.replace_all(&null_enable, ")").to_string())
        .unwrap_or_else(|_| null_enable);

    constraint_enable
        .lines()
        .filter(|line| !line.trim_start().to_uppercase().starts_with("DELIMITER"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(clippy::too_many_arguments)]
fn collect_statement(
    statement: Statement,
    tables: &mut Vec<ParsedTable>,
    table_index: &mut HashMap<String, usize>,
    table_comments: &mut HashMap<String, String>,
    column_comments: &mut HashMap<(String, String), String>,
) {
    match statement {
        Statement::CreateTable(create_table) => {
            let name = last_ident(&create_table.name);
            if name.is_empty() || table_index.contains_key(&name) {
                return;
            }

            let mut columns: Vec<ParsedColumn> = Vec::new();
            for column_def in &create_table.columns {
                columns.push(parse_column_def(column_def));
            }

            let mut indexes: Vec<ParsedIndex> = Vec::new();
            for constraint in &create_table.constraints {
                match constraint {
                    TableConstraint::PrimaryKey(constraint) => {
                        let pk_names = constraint_column_names(&constraint.columns);
                        for column in columns.iter_mut() {
                            if pk_names.iter().any(|item| item.eq_ignore_ascii_case(&column.name)) {
                                column.primary_key = true;
                                column.nullable = false;
                            }
                        }
                    }
                    TableConstraint::Index(constraint) => {
                        indexes.push(ParsedIndex {
                            name: constraint.name.as_ref().map(|ident| ident.value.clone()).unwrap_or_default(),
                            columns: constraint_column_names(&constraint.columns),
                            unique: false,
                        });
                    }
                    TableConstraint::Unique(constraint) => {
                        let unique_names = constraint_column_names(&constraint.columns);
                        indexes.push(ParsedIndex {
                            name: unique_index_display_name(constraint),
                            columns: unique_names,
                            unique: true,
                        });
                    }
                    _ => {}
                }
            }

            table_index.insert(name.clone(), tables.len());
            tables.push(ParsedTable {
                name,
                comment: extract_table_comment(&create_table),
                columns,
                indexes,
            });
        }
        Statement::CreateIndex(create_index) => {
            let table = last_ident(&create_index.table_name);
            let index_name = create_index
                .name
                .as_ref()
                .map(last_ident)
                .unwrap_or_default();
            let index_columns = create_index.columns.iter().map(index_column_name).collect();
            if let Some(position) = table_index.get(&table) {
                tables[*position].indexes.push(ParsedIndex {
                    name: index_name,
                    columns: index_columns,
                    unique: create_index.unique,
                });
            }
        }
        // pg_dump 把主键放在 ALTER TABLE ... ADD CONSTRAINT ... PRIMARY KEY 里
        Statement::AlterTable(alter_table) => {
            let table = last_ident(&alter_table.name);
            for operation in &alter_table.operations {
                if let sqlparser::ast::AlterTableOperation::AddConstraint {
                    constraint: TableConstraint::PrimaryKey(constraint),
                    ..
                } = operation
                {
                    let pk_names = constraint_column_names(&constraint.columns);
                    if let Some(position) = table_index.get(&table) {
                        for column in tables[*position].columns.iter_mut() {
                            if pk_names
                                .iter()
                                .any(|item| item.eq_ignore_ascii_case(&column.name))
                            {
                                column.primary_key = true;
                                column.nullable = false;
                            }
                        }
                    }
                }
            }
        }
        Statement::Comment {
            object_type,
            object_name,
            comment: Some(text),
            ..
        } => match object_type {
            sqlparser::ast::CommentObject::Table => {
                let name = last_ident(&object_name);
                table_comments.insert(name, text);
            }
            sqlparser::ast::CommentObject::Column => {
                // COMMENT ON TABLE.COLUMN：取表与列两段
                let parts = object_name_parts(&object_name);
                if parts.len() >= 2 {
                    let table = parts[parts.len() - 2].clone();
                    let column = parts[parts.len() - 1].clone();
                    column_comments.insert((table, column), text);
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn parse_column_def(column_def: &sqlparser::ast::ColumnDef) -> ParsedColumn {
    let mut nullable = true;
    let mut default = None;
    let mut comment = String::new();
    let mut primary_key = false;

    for option_def in &column_def.options {
        match &option_def.option {
            ColumnOption::NotNull => nullable = false,
            ColumnOption::Default(expression) => default = Some(expression.to_string()),
            ColumnOption::PrimaryKey(_) => {
                primary_key = true;
                nullable = false;
            }
            ColumnOption::Comment(text) => comment = text.clone(),
            _ => {}
        }
    }

    ParsedColumn {
        name: column_def.name.value.clone(),
        data_type: column_def.data_type.to_string().to_lowercase(),
        nullable,
        default,
        comment,
        primary_key,
        ordinal: None,
    }
}

/// 约束名：CONSTRAINT 名优先，回退到 MySQL `UNIQUE KEY <index_name>` 的索引名
fn constraint_display_name(constraint: &sqlparser::ast::UniqueConstraint) -> String {
    constraint
        .name
        .as_ref()
        .or(constraint.index_name.as_ref())
        .map(|ident| ident.value.clone())
        .unwrap_or_default()
}

fn unique_index_display_name(constraint: &sqlparser::ast::UniqueConstraint) -> String {
    constraint_display_name(constraint)
}

/// MySQL 表级选项里的注释（COMMENT='...'）
fn extract_table_comment(create_table: &sqlparser::ast::CreateTable) -> String {
    let options: &[sqlparser::ast::SqlOption] = match &create_table.table_options {
        sqlparser::ast::CreateTableOptions::Plain(options)
        | sqlparser::ast::CreateTableOptions::Options(options)
        | sqlparser::ast::CreateTableOptions::With(options) => options,
        _ => &[],
    };

    for option in options {
        if let sqlparser::ast::SqlOption::Comment(comment) = option {
            return match comment {
                sqlparser::ast::CommentDef::WithEq(text) | sqlparser::ast::CommentDef::WithoutEq(text) => {
                    text.clone()
                }
            };
        }
    }
    String::new()
}

fn constraint_column_names<T>(columns: &[T]) -> Vec<String>
where
    T: ConstraintColumn,
{
    columns.iter().map(|column| column.constraint_name()).collect()
}

/// 让不同的列集合类型（IndexColumn / OrderByExpr 等）都能取到列名
trait ConstraintColumn {
    fn constraint_name(&self) -> String;
}

impl ConstraintColumn for sqlparser::ast::IndexColumn {
    fn constraint_name(&self) -> String {
        match &self.column.expr {
            sqlparser::ast::Expr::Identifier(ident) => ident.value.clone(),
            other => other.to_string(),
        }
    }
}

impl ConstraintColumn for sqlparser::ast::OrderByExpr {
    fn constraint_name(&self) -> String {
        match &self.expr {
            sqlparser::ast::Expr::Identifier(ident) => ident.value.clone(),
            other => other.to_string(),
        }
    }
}

fn index_column_name(column: &sqlparser::ast::IndexColumn) -> String {
    column.constraint_name()
}

/// 取对象名的最后一段（剥离库名/schema 前缀与引号）
fn last_ident(name: &ObjectName) -> String {
    object_name_parts(name)
        .last()
        .cloned()
        .unwrap_or_default()
}

fn object_name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|part| match part {
            sqlparser::ast::ObjectNamePart::Identifier(ident) => Some(ident.value.clone()),
            _ => None,
        })
        .collect()
}

fn apply_comments(
    tables: &mut [ParsedTable],
    table_comments: &HashMap<String, String>,
    column_comments: &HashMap<(String, String), String>,
) {
    for table in tables.iter_mut() {
        if let Some(text) = table_comments.get(&table.name) {
            table.comment = text.clone();
        }
        for column in table.columns.iter_mut() {
            if let Some(text) = column_comments.get(&(table.name.clone(), column.name.clone())) {
                column.comment = text.clone();
            }
            // 注释匹配大小写可能不一致（Oracle 未加引号的标识符大写），兜底一次忽略大小写
            if column.comment.is_empty() {
                if let Some((_, text)) = column_comments
                    .iter()
                    .find(|((table_name, column_name), _)| {
                        table_name.eq_ignore_ascii_case(&table.name)
                            && column_name.eq_ignore_ascii_case(&column.name)
                    })
                {
                    column.comment = text.clone();
                }
            }
        }
    }
}

// ===== 智能查询结果解析（JSON / TSV）=====

/// 识别的字段键（全部小写归一化后匹配）
const FIELD_TABLE: &[&str] = &["table_name", "table", "表名"];
const FIELD_COLUMN: &[&str] = &["column_name", "column", "field", "name", "字段名"];
const FIELD_TYPE: &[&str] = &["data_type", "type", "数据类型"];
const FIELD_NULLABLE: &[&str] = &["is_nullable", "nullable", "可空"];
const FIELD_DEFAULT: &[&str] = &["column_default", "default", "默认值"];
const FIELD_ORDINAL: &[&str] = &["ordinal_position", "position", "ordinal", "序号"];
const FIELD_COMMENT: &[&str] = &["column_comment", "comment", "注释"];
const FIELD_PK: &[&str] = &["primary_key", "column_key", "pk", "主键"];

/// 解析智能查询粘贴的结果：JSON 数组或带表头 TSV
pub fn parse_query_result(text: &str) -> Result<ParsedSchema, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("没有可解析的内容".to_string());
    }

    let rows: Vec<HashMap<String, String>> = if trimmed.starts_with('[') {
        parse_json_rows(trimmed)?
    } else {
        parse_tsv_rows(trimmed)?
    };

    build_schema(rows)
}

fn normalize_key(key: &str) -> String {
    key.trim()
        .trim_matches(|c| c == '`' || c == '"' || c == '\'')
        .to_lowercase()
}

fn field_of<'a>(aliases: &[&str], row: &'a HashMap<String, String>) -> Option<&'a String> {
    aliases.iter().find_map(|alias| row.get(*alias))
}

fn truthy(value: &str) -> bool {
    matches!(value.trim().to_uppercase().as_str(), "Y" | "YES" | "1" | "TRUE" | "PRI" | "是")
}

fn parse_json_rows(text: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("不是合法的 JSON: {error}"))?;
    let items = match value {
        serde_json::Value::Array(items) => items,
        _ => return Err("JSON 应该是对象数组（[{...}, ...]）".to_string()),
    };

    let mut rows = Vec::new();
    for item in items {
        let serde_json::Value::Object(map) = item else {
            continue;
        };
        let mut row = HashMap::new();
        for (key, value) in map {
            let normalized = normalize_key(&key);
            let text = match value {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(text) => text,
                other => other.to_string(),
            };
            row.insert(normalized, text);
        }
        rows.push(row);
    }
    Ok(rows)
}

fn parse_tsv_rows(text: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header_line = lines.next().ok_or("内容为空")?;
    let headers: Vec<String> = header_line.split('\t').map(normalize_key).collect();
    if headers.iter().all(|header| header.is_empty()) {
        return Err("表头无法识别，请粘贴带表头的查询结果".to_string());
    }

    let mut rows = Vec::new();
    for line in lines {
        let values: Vec<&str> = line.split('\t').collect();
        let mut row = HashMap::new();
        for (index, header) in headers.iter().enumerate() {
            if header.is_empty() {
                continue;
            }
            let value = values.get(index).copied().unwrap_or("");
            row.insert(header.clone(), value.trim().to_string());
        }
        rows.push(row);
    }
    Ok(rows)
}

fn build_schema(rows: Vec<HashMap<String, String>>) -> Result<ParsedSchema, String> {
    let mut tables: Vec<ParsedTable> = Vec::new();
    let mut table_index: HashMap<String, usize> = HashMap::new();
    let mut used_rows = 0usize;

    for row in &rows {
        let Some(table_name) = field_of(FIELD_TABLE, row).cloned() else {
            continue;
        };
        let Some(column_name) = field_of(FIELD_COLUMN, row).cloned() else {
            continue;
        };
        if table_name.is_empty() || column_name.is_empty() {
            continue;
        }
        used_rows += 1;

        let position = match table_index.get(&table_name) {
            Some(index) => *index,
            None => {
                table_index.insert(table_name.clone(), tables.len());
                tables.push(ParsedTable {
                    name: table_name.clone(),
                    ..Default::default()
                });
                tables.len() - 1
            }
        };

        let data_type = field_of(FIELD_TYPE, row).cloned().unwrap_or_default();
        let nullable = match field_of(FIELD_NULLABLE, row) {
            Some(value) => !matches!(value.trim().to_uppercase().as_str(), "NO" | "N" | "0" | "FALSE" | "否"),
            None => true,
        };
        let default = field_of(FIELD_DEFAULT, row)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let comment = field_of(FIELD_COMMENT, row).cloned().unwrap_or_default();
        let primary_key = field_of(FIELD_PK, row).is_some_and(|value| truthy(value));
        let ordinal = field_of(FIELD_ORDINAL, row)
            .and_then(|value| value.trim().parse::<i64>().ok());

        tables[position].columns.push(ParsedColumn {
            name: column_name,
            data_type: data_type.to_lowercase(),
            nullable,
            default,
            comment,
            primary_key,
            ordinal,
        });
    }

    if used_rows == 0 || tables.is_empty() {
        return Err(
            "没有识别出任何表结构行，请检查粘贴内容是否包含 table_name / column_name 等列"
                .to_string(),
        );
    }

    // 按序号稳定排序每张表的字段；无序号的保持出现顺序
    for table in tables.iter_mut() {
        table.columns.sort_by_key(|column| column.ordinal.unwrap_or(i64::MAX));
        for column in table.columns.iter_mut() {
            column.ordinal = None;
        }
    }

    Ok(ParsedSchema {
        tables,
        skipped_statements: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MYSQL_DDL: &str = r#"
-- ----------------------------
-- Table structure for patient
-- ----------------------------
/*!40101 SET NAMES utf8mb4 */;
SET NAMES utf8mb4;
LOCK TABLES `patient` WRITE;
DROP TABLE IF EXISTS `patient`;
CREATE TABLE `patient` (
  `id` int(11) NOT NULL AUTO_INCREMENT COMMENT '主键',
  `name` varchar(50) NOT NULL COMMENT '姓名',
  `id_card` varchar(18) DEFAULT NULL,
  `ward_no` varchar(20) DEFAULT '' COMMENT '病区',
  `created_at` datetime DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_ward` (`ward_no`),
  UNIQUE KEY `uk_id_card` (`id_card`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='住院病人表';
UNLOCK TABLES;
"#;

    const PG_DDL: &str = r#"
SET statement_timeout = 0;
SELECT pg_catalog.set_config('search_path', '', false);

CREATE TABLE public.visit (
    id bigint NOT NULL,
    patient_id bigint NOT NULL,
    visit_type varchar(20) DEFAULT 'outpatient'::character varying,
    memo text
);
COMMENT ON TABLE public.visit IS '就诊记录';
COMMENT ON COLUMN public.visit.visit_type IS '就诊类型';

CREATE INDEX idx_visit_patient ON public.visit USING btree (patient_id);
ALTER TABLE ONLY public.visit ADD CONSTRAINT visit_pkey PRIMARY KEY (id);
"#;

    const ORACLE_DDL: &str = r#"
  CREATE TABLE "HIS"."INPATIENT"
   (    "ID" NUMBER(10,0) NOT NULL ENABLE,
        "NAME" VARCHAR2(100),
        "BED_NO" VARCHAR2(20) DEFAULT '',
         CONSTRAINT "INPATIENT_PK" PRIMARY KEY ("ID")
   ) ;

  COMMENT ON TABLE "HIS"."INPATIENT" IS '住院登记';
  COMMENT ON COLUMN "HIS"."INPATIENT"."BED_NO" IS '床位号';
"#;

    #[test]
    fn mysql_ddl_parses_with_noise_skipped() {
        let outcome = parse_ddl(MYSQL_DDL, "mysql").expect("mysql ddl");
        assert_eq!(outcome.schema.tables.len(), 1);
        assert!(outcome.schema.skipped_statements <= 2, "SET/UNLOCK may skip");

        let table = &outcome.schema.tables[0];
        assert_eq!(table.name, "patient");
        assert_eq!(table.comment, "住院病人表");
        assert_eq!(table.columns.len(), 5);

        let id = &table.columns[0];
        assert_eq!(id.name, "id");
        assert!(id.primary_key);
        assert!(!id.nullable);
        assert_eq!(id.comment, "主键");
        assert!(id.data_type.starts_with("int"));

        let name = &table.columns[1];
        assert!(!name.nullable);
        assert_eq!(name.data_type, "varchar(50)");

        let created = &table.columns[4];
        assert!(created.default.is_some());

        assert_eq!(table.indexes.len(), 2, "KEY + UNIQUE KEY");
        assert!(table.indexes.iter().any(|index| index.name == "uk_id_card" && index.unique));
    }

    #[test]
    fn pg_ddl_parses_with_comments() {
        let outcome = parse_ddl(PG_DDL, "pg").expect("pg ddl");
        assert_eq!(outcome.schema.tables.len(), 1);
        assert!(outcome.skipped_statements <= 2, "SET/SELECT noise");

        let table = &outcome.schema.tables[0];
        assert_eq!(table.name, "visit");
        assert_eq!(table.comment, "就诊记录");

        let visit_type = table.columns.iter().find(|c| c.name == "visit_type").expect("column");
        assert_eq!(visit_type.comment, "就诊类型");
        assert_eq!(visit_type.data_type, "varchar(20)");
        assert!(visit_type.default.is_some());

        let id = table.columns.iter().find(|c| c.name == "id").expect("column");
        assert!(id.primary_key, "PK comes from ALTER ... but alter is skipped");
        assert!(!id.nullable);
        assert_eq!(table.indexes.len(), 1);
        assert_eq!(table.indexes[0].name, "idx_visit_patient");
    }

    #[test]
    fn oracle_ddl_parses_with_comments() {
        let outcome = parse_ddl(ORACLE_DDL, "oracle").expect("oracle ddl");
        assert_eq!(outcome.schema.tables.len(), 1);

        let table = &outcome.schema.tables[0];
        assert_eq!(table.name, "INPATIENT");
        assert_eq!(table.comment, "住院登记");

        let bed = table.columns.iter().find(|c| c.name == "BED_NO").expect("column");
        assert_eq!(bed.comment, "床位号");
        assert!(bed.nullable);

        let id = table.columns.iter().find(|c| c.name == "ID").expect("column");
        assert!(id.primary_key);
        assert!(!id.nullable);
        assert!(id.data_type.starts_with("number"));
    }

    #[test]
    fn query_result_parses_json_and_tsv() {
        let json = r#"[{"TABLE_NAME":"visit","COLUMN_NAME":"id","DATA_TYPE":"bigint","IS_NULLABLE":"NO","COLUMN_DEFAULT":null,"ORDINAL_POSITION":1,"COLUMN_COMMENT":"主键","COLUMN_KEY":"PRI"},
            {"TABLE_NAME":"visit","COLUMN_NAME":"memo","DATA_TYPE":"text","IS_NULLABLE":"YES","ORDINAL_POSITION":2}]"#;
        let schema = parse_query_result(json).expect("json");
        assert_eq!(schema.tables.len(), 1);
        let table = &schema.tables[0];
        assert_eq!(table.name, "visit");
        assert_eq!(table.columns.len(), 2);
        assert!(table.columns[0].primary_key);
        assert!(!table.columns[0].nullable);
        assert_eq!(table.columns[0].comment, "主键");

        let tsv = "TABLE_NAME\tCOLUMN_NAME\tDATA_TYPE\tIS_NULLABLE\tORDINAL_POSITION\nvisit\tid\tbigint\tNO\t1\nvisit\tmemo\ttext\tYES\t2";
        let schema = parse_query_result(tsv).expect("tsv");
        assert_eq!(schema.tables[0].columns.len(), 2);
        assert_eq!(schema.tables[0].columns[0].name, "id");
    }

    #[test]
    fn query_result_rejects_garbage() {
        assert!(parse_query_result("").is_err());
        assert!(parse_query_result("随便一句话").is_err());
        assert!(parse_query_result("[{\"foo\":\"bar\"}]").is_err());
        assert!(parse_ddl("SELECT 1", "mysql").is_err());
    }
}


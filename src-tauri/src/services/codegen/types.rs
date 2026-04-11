use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModelField {
    pub name: String,
    pub json_name: String,
    pub go_type: String,
    pub ts_type: String,
    pub label: String,
    pub required: bool,
    pub max_length: Option<usize>,
    pub is_optional: bool,
    pub is_json: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ParsedEntity {
    pub name: String,
    pub snake_name: String,
    pub table_name: String,
    pub module_name: String,
    pub fields: Vec<ModelField>,
    pub search_fields: Vec<ModelField>,
    pub param_fields: Vec<ModelField>,
    pub rpc_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedField {
    pub name: String,
    pub go_name: String,
    pub json_name: String,
    pub go_type: String,
    pub db_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub validate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTable {
    pub table_name: String,
    pub entity_name: String,
    pub entity_label: Option<String>,
    pub fields: Vec<ParsedField>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuditType {
    FullAudited,
    FullTracked,
    CreationAudited,
    CreationTracked,
    None,
}

impl Default for AuditType {
    fn default() -> Self {
        Self::FullAudited
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoCodeGenConfig {
    pub entity_name: String,
    pub module_path: String,
    pub table_name: String,
    pub table_alias: String,
    pub rpc_path: String,
    pub go_module_prefix: String,
    pub output_dir: String,
    pub fields: Vec<ParsedField>,
    pub audit_type: AuditType,
    pub enable_find_page: bool,
    pub enable_create: bool,
    pub enable_update: bool,
    pub enable_delete: bool,
    pub enable_delete_many: bool,
    pub enable_sort: bool,
    pub enable_audit_user_names: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedGoCrudResult {
    pub generated_files: Vec<String>,
    pub module_registration_snippet: String,
}

/// orm.BaseModel is used for Bun table metadata and does not contribute API fields.
pub const BASE_MODEL_FIELDS: &[(&str, &str, &str)] = &[];

/// Fields contributed by orm.CreationTrackedModel.
pub const CREATION_TRACKED_FIELDS: &[(&str, &str, &str)] = &[
    ("created_at", "createdAt", "string"),
    ("created_by", "createdBy", "string"),
    ("created_by_name", "createdByName", "string"),
];

/// Fields contributed by orm.FullTrackedModel.
pub const FULL_TRACKED_FIELDS: &[(&str, &str, &str)] = &[
    ("created_at", "createdAt", "string"),
    ("created_by", "createdBy", "string"),
    ("created_by_name", "createdByName", "string"),
    ("updated_at", "updatedAt", "string"),
    ("updated_by", "updatedBy", "string"),
    ("updated_by_name", "updatedByName", "string"),
];

/// Fields contributed by orm.CreationAuditedModel.
pub const CREATION_AUDITED_FIELDS: &[(&str, &str, &str)] = &[
    ("id", "id", "string"),
    ("created_at", "createdAt", "string"),
    ("created_by", "createdBy", "string"),
    ("created_by_name", "createdByName", "string"),
];

/// Fields contributed by orm.FullAuditedModel.
pub const FULL_AUDITED_FIELDS: &[(&str, &str, &str)] = &[
    ("id", "id", "string"),
    ("created_at", "createdAt", "string"),
    ("created_by", "createdBy", "string"),
    ("created_by_name", "createdByName", "string"),
    ("updated_at", "updatedAt", "string"),
    ("updated_by", "updatedBy", "string"),
    ("updated_by_name", "updatedByName", "string"),
];

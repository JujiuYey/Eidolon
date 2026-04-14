use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportType {
    #[default]
    Stdio,
    StreamableHttp,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpDiscoveredTool {
    pub name: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub input_schema: Value,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub auto_approve: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpDiscoveredPrompt {
    pub name: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpDiscoveredResource {
    pub uri: String,

    pub name: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub mime_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpDiscoveredResourceTemplate {
    pub uri_template: String,

    pub name: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub mime_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpServiceDiscovery {
    #[serde(default)]
    pub tested_at: String,

    #[serde(default)]
    pub server_name: String,

    #[serde(default)]
    pub server_version: String,

    #[serde(default)]
    pub instructions: String,

    #[serde(default)]
    pub supports_tools: bool,

    #[serde(default)]
    pub supports_prompts: bool,

    #[serde(default)]
    pub supports_resources: bool,

    #[serde(default)]
    pub tools: Vec<McpDiscoveredTool>,

    #[serde(default)]
    pub prompts: Vec<McpDiscoveredPrompt>,

    #[serde(default)]
    pub resources: Vec<McpDiscoveredResource>,

    #[serde(default)]
    pub resource_templates: Vec<McpDiscoveredResourceTemplate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpService {
    #[serde(default)]
    pub id: String,

    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub transport_type: McpTransportType,

    #[serde(default)]
    pub command: String,

    #[serde(default)]
    pub args: String,

    #[serde(default)]
    pub env: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub long_running: bool,

    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    #[serde(default)]
    pub discovery: Option<McpServiceDiscovery>,
}

impl Default for McpService {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            enabled: true,
            transport_type: McpTransportType::default(),
            command: String::new(),
            args: String::new(),
            env: String::new(),
            url: String::new(),
            long_running: false,
            timeout_seconds: default_timeout_seconds(),
            discovery: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout_seconds() -> u64 {
    60
}

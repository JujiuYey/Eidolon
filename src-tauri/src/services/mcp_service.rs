use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use rmcp::{
    model::{ClientInfo, Prompt, PromptArgument, Resource, ResourceTemplate, ServerInfo, Tool},
    service::Peer,
    transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use tokio::process::Command;
use tokio::time::timeout;

use crate::models::mcp_service::{
    McpDiscoveredPrompt, McpDiscoveredResource, McpDiscoveredResourceTemplate, McpDiscoveredTool,
    McpPromptArgument, McpService, McpServiceDiscovery, McpTransportType,
};

pub async fn discover_service(service: &McpService) -> Result<McpServiceDiscovery, String> {
    validate_service_for_discovery(service)?;

    let timeout_seconds = service.timeout_seconds.max(1);
    timeout(Duration::from_secs(timeout_seconds), async {
        match service.transport_type {
            McpTransportType::Stdio => discover_stdio_service(service).await,
            McpTransportType::StreamableHttp => discover_streamable_http_service(service).await,
        }
    })
    .await
    .map_err(|_| format!("连接超时（>{timeout_seconds} 秒）"))?
}

async fn discover_stdio_service(service: &McpService) -> Result<McpServiceDiscovery, String> {
    let args = parse_args(&service.args);
    let env = parse_env(&service.env)?;

    let (transport, stderr) =
        TokioChildProcess::builder(Command::new(service.command.trim()).configure(|command| {
            command.args(&args);
            command.envs(&env);
        }))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("启动 MCP 进程失败: {error}"))?;

    let client = match ClientInfo::default().serve(transport).await {
        Ok(client) => client,
        Err(error) => {
            return Err(append_stderr(format!("连接 MCP 服务失败: {error}"), stderr).await);
        }
    };

    let discovery = collect_discovery_from_peer(
        client.peer(),
        client.peer_info(),
        service.discovery.as_ref(),
    )
    .await;

    let _ = client.cancel().await;

    match discovery {
        Ok(result) => Ok(result),
        Err(error) => Err(append_stderr(error, stderr).await),
    }
}

async fn discover_streamable_http_service(
    service: &McpService,
) -> Result<McpServiceDiscovery, String> {
    let transport = StreamableHttpClientTransport::from_uri(service.url.trim().to_string());
    let client = ClientInfo::default()
        .serve(transport)
        .await
        .map_err(|error| format!("连接 MCP 服务失败: {error}"))?;

    let discovery = collect_discovery_from_peer(
        client.peer(),
        client.peer_info(),
        service.discovery.as_ref(),
    )
    .await;
    let _ = client.cancel().await;
    discovery
}

async fn collect_discovery_from_peer(
    peer: &Peer<RoleClient>,
    peer_info: Option<&ServerInfo>,
    existing_discovery: Option<&McpServiceDiscovery>,
) -> Result<McpServiceDiscovery, String> {
    let server_info = peer_info.cloned().unwrap_or_default();
    let supports_tools = server_info.capabilities.tools.is_some();
    let supports_prompts = server_info.capabilities.prompts.is_some();
    let supports_resources = server_info.capabilities.resources.is_some();

    let tools = if supports_tools {
        peer.list_all_tools()
            .await
            .map_err(|error| format!("读取工具列表失败: {error}"))?
    } else {
        Vec::new()
    };

    let prompts = if supports_prompts {
        peer.list_all_prompts()
            .await
            .map_err(|error| format!("读取提示列表失败: {error}"))?
    } else {
        Vec::new()
    };

    let resources = if supports_resources {
        peer.list_all_resources()
            .await
            .map_err(|error| format!("读取资源列表失败: {error}"))?
    } else {
        Vec::new()
    };

    let resource_templates = if supports_resources {
        peer.list_all_resource_templates()
            .await
            .map_err(|error| format!("读取资源模板失败: {error}"))?
    } else {
        Vec::new()
    };

    Ok(McpServiceDiscovery {
        tested_at: Utc::now().to_rfc3339(),
        server_name: server_info.server_info.name,
        server_version: server_info.server_info.version,
        instructions: server_info.instructions.unwrap_or_default(),
        supports_tools,
        supports_prompts,
        supports_resources,
        tools: merge_tool_preferences(existing_discovery, tools),
        prompts: prompts.into_iter().map(map_prompt).collect(),
        resources: resources.into_iter().map(map_resource).collect(),
        resource_templates: resource_templates
            .into_iter()
            .map(map_resource_template)
            .collect(),
    })
}

fn merge_tool_preferences(
    existing_discovery: Option<&McpServiceDiscovery>,
    tools: Vec<Tool>,
) -> Vec<McpDiscoveredTool> {
    let existing_preferences = existing_discovery
        .map(|discovery| {
            discovery
                .tools
                .iter()
                .map(|tool| (tool.name.clone(), (tool.enabled, tool.auto_approve)))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    tools
        .into_iter()
        .map(|tool| {
            let (enabled, auto_approve) = existing_preferences
                .get(tool.name.as_ref())
                .copied()
                .unwrap_or((true, false));

            McpDiscoveredTool {
                name: tool.name.to_string(),
                title: tool.title.clone().unwrap_or_default(),
                description: tool
                    .description
                    .clone()
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                input_schema: tool.schema_as_json_value(),
                enabled,
                auto_approve,
            }
        })
        .collect()
}

fn map_prompt(prompt: Prompt) -> McpDiscoveredPrompt {
    McpDiscoveredPrompt {
        name: prompt.name,
        title: prompt.title.unwrap_or_default(),
        description: prompt.description.unwrap_or_default(),
        arguments: prompt
            .arguments
            .unwrap_or_default()
            .into_iter()
            .map(map_prompt_argument)
            .collect(),
    }
}

fn map_prompt_argument(argument: PromptArgument) -> McpPromptArgument {
    McpPromptArgument {
        name: argument.name,
        title: argument.title.unwrap_or_default(),
        description: argument.description.unwrap_or_default(),
        required: argument.required.unwrap_or(false),
    }
}

fn map_resource(resource: Resource) -> McpDiscoveredResource {
    McpDiscoveredResource {
        uri: resource.uri.clone(),
        name: resource.name.clone(),
        title: resource.title.clone().unwrap_or_default(),
        description: resource.description.clone().unwrap_or_default(),
        mime_type: resource.mime_type.clone().unwrap_or_default(),
    }
}

fn map_resource_template(resource_template: ResourceTemplate) -> McpDiscoveredResourceTemplate {
    McpDiscoveredResourceTemplate {
        uri_template: resource_template.uri_template.clone(),
        name: resource_template.name.clone(),
        title: resource_template.title.clone().unwrap_or_default(),
        description: resource_template.description.clone().unwrap_or_default(),
        mime_type: resource_template.mime_type.clone().unwrap_or_default(),
    }
}

fn validate_service_for_discovery(service: &McpService) -> Result<(), String> {
    match service.transport_type {
        McpTransportType::Stdio => {
            if service.command.trim().is_empty() {
                return Err("请先填写 STDIO 命令".to_string());
            }
        }
        McpTransportType::StreamableHttp => {
            if service.url.trim().is_empty() {
                return Err("请先填写 Streamable HTTP 地址".to_string());
            }
        }
    }

    Ok(())
}

fn parse_args(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_env(raw: &str) -> Result<HashMap<String, String>, String> {
    let mut env = HashMap::new();

    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "环境变量第 {} 行格式不正确，应为 KEY=value",
                index + 1
            ));
        };

        let normalized_key = key.trim();
        if normalized_key.is_empty() {
            return Err(format!("环境变量第 {} 行的 KEY 不能为空", index + 1));
        }

        env.insert(normalized_key.to_string(), value.to_string());
    }

    Ok(env)
}

async fn append_stderr(error: String, stderr: Option<tokio::process::ChildStderr>) -> String {
    if let Some(mut stderr) = stderr {
        let mut output = String::new();
        use tokio::io::AsyncReadExt;
        let _ = stderr.read_to_string(&mut output).await;
        let trimmed = output.trim();
        if !trimmed.is_empty() {
            return format!("{error}\nSTDERR: {trimmed}");
        }
    }

    error
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rmcp::{
        model::AnnotateAble,
        model::{
            Implementation, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
            ListToolsResult, PaginatedRequestParams, Prompt, PromptArgument, RawResource,
            RawResourceTemplate, ServerCapabilities, ServerInfo, Tool,
        },
        service::RequestContext,
        ErrorData, RoleServer, ServerHandler, ServiceExt,
    };
    use serde_json::Map;

    use super::{collect_discovery_from_peer, parse_env, ClientInfo};
    use crate::models::mcp_service::McpServiceDiscovery;

    #[derive(Clone, Default)]
    struct TestDiscoveryServer;

    impl ServerHandler for TestDiscoveryServer {
        fn get_info(&self) -> ServerInfo {
            ServerInfo {
                capabilities: ServerCapabilities::builder()
                    .enable_tools()
                    .enable_prompts()
                    .enable_resources()
                    .build(),
                server_info: Implementation {
                    name: "test-mcp".to_string(),
                    version: "0.1.0".to_string(),
                    ..Default::default()
                },
                instructions: Some("Use this server for tests".to_string()),
                ..Default::default()
            }
        }

        async fn list_tools(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListToolsResult, ErrorData> {
            Ok(ListToolsResult::with_all_items(vec![Tool::new(
                "search_docs",
                "Search the docs",
                Arc::new(Map::new()),
            )]))
        }

        async fn list_prompts(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListPromptsResult, ErrorData> {
            Ok(ListPromptsResult::with_all_items(vec![Prompt::new(
                "review_code",
                Some("Review a patch"),
                Some(vec![PromptArgument {
                    name: "diff".to_string(),
                    title: Some("Diff".to_string()),
                    description: Some("Unified diff".to_string()),
                    required: Some(true),
                }]),
            )]))
        }

        async fn list_resources(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListResourcesResult, ErrorData> {
            Ok(ListResourcesResult::with_all_items(vec![RawResource {
                uri: "file:///README.md".to_string(),
                name: "README".to_string(),
                title: None,
                description: Some("Project README".to_string()),
                mime_type: Some("text/markdown".to_string()),
                size: None,
                icons: None,
                meta: None,
            }
            .no_annotation()]))
        }

        async fn list_resource_templates(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListResourceTemplatesResult, ErrorData> {
            Ok(ListResourceTemplatesResult::with_all_items(vec![
                RawResourceTemplate {
                    uri_template: "file:///src/{path}".to_string(),
                    name: "Source files".to_string(),
                    title: Some("Source files".to_string()),
                    description: Some("Read source files by path".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    icons: None,
                }
                .no_annotation(),
            ]))
        }
    }

    #[tokio::test]
    async fn collect_discovery_reads_all_capabilities_and_preserves_tool_preferences() {
        let (client_to_server, server_from_client) = tokio::io::duplex(8192);
        let (server_to_client, client_from_server) = tokio::io::duplex(8192);

        tokio::spawn(async move {
            let service = TestDiscoveryServer
                .serve((server_from_client, server_to_client))
                .await
                .expect("server should start");
            service.waiting().await.expect("server should stay healthy");
        });

        let client = ClientInfo::default()
            .serve((client_from_server, client_to_server))
            .await
            .expect("client should connect");

        let existing = McpServiceDiscovery {
            tools: vec![crate::models::mcp_service::McpDiscoveredTool {
                name: "search_docs".to_string(),
                enabled: false,
                auto_approve: true,
                ..Default::default()
            }],
            ..Default::default()
        };

        let discovery =
            collect_discovery_from_peer(client.peer(), client.peer_info(), Some(&existing))
                .await
                .expect("discovery should succeed");

        assert_eq!(discovery.server_name, "test-mcp");
        assert_eq!(discovery.server_version, "0.1.0");
        assert!(discovery.supports_tools);
        assert!(discovery.supports_prompts);
        assert!(discovery.supports_resources);
        assert_eq!(discovery.tools.len(), 1);
        assert_eq!(discovery.tools[0].name, "search_docs");
        assert!(!discovery.tools[0].enabled);
        assert!(discovery.tools[0].auto_approve);
        assert_eq!(discovery.prompts[0].name, "review_code");
        assert_eq!(discovery.prompts[0].arguments[0].name, "diff");
        assert!(discovery.prompts[0].arguments[0].required);
        assert_eq!(discovery.resources[0].uri, "file:///README.md");
        assert_eq!(
            discovery.resource_templates[0].uri_template,
            "file:///src/{path}"
        );

        let _ = client.cancel().await;
    }

    #[test]
    fn parse_env_rejects_invalid_lines() {
        let error = parse_env("GOOD=value\nBAD").expect_err("invalid env should fail");
        assert!(error.contains("第 2 行"));
    }
}

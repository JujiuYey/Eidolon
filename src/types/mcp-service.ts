export type McpTransportType = 'stdio' | 'streamable_http';

export interface McpDiscoveredTool {
  name: string;
  title: string;
  description: string;
  input_schema: unknown;
  enabled: boolean;
  auto_approve: boolean;
}

export interface McpPromptArgument {
  name: string;
  title: string;
  description: string;
  required: boolean;
}

export interface McpDiscoveredPrompt {
  name: string;
  title: string;
  description: string;
  arguments: McpPromptArgument[];
}

export interface McpDiscoveredResource {
  uri: string;
  name: string;
  title: string;
  description: string;
  mime_type: string;
}

export interface McpDiscoveredResourceTemplate {
  uri_template: string;
  name: string;
  title: string;
  description: string;
  mime_type: string;
}

export interface McpServiceDiscovery {
  tested_at: string;
  server_name: string;
  server_version: string;
  instructions: string;
  supports_tools: boolean;
  supports_prompts: boolean;
  supports_resources: boolean;
  tools: McpDiscoveredTool[];
  prompts: McpDiscoveredPrompt[];
  resources: McpDiscoveredResource[];
  resource_templates: McpDiscoveredResourceTemplate[];
}

export interface McpService {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  transport_type: McpTransportType;
  command: string;
  args: string;
  env: string;
  url: string;
  long_running: boolean;
  timeout_seconds: number;
  discovery?: McpServiceDiscovery | null;
}

import { invoke } from '@tauri-apps/api/core';
import type {
  McpService,
  McpServiceDiscovery,
} from '@/types/mcp-service';

export async function listMcpServices(): Promise<McpService[]> {
  return invoke<McpService[]>('list_mcp_services');
}

export async function upsertMcpService(service: McpService): Promise<string> {
  return invoke<string>('upsert_mcp_service', { service });
}

export async function deleteMcpService(serviceId: string): Promise<string> {
  return invoke<string>('delete_mcp_service', { serviceId });
}

export async function discoverMcpService(service: McpService): Promise<McpServiceDiscovery> {
  return invoke<McpServiceDiscovery>('discover_mcp_service', { service });
}

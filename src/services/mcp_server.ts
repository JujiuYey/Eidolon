import { invoke } from '@tauri-apps/api/core';

/** 对外 MCP 服务的设置与运行状态 */
export interface McpServerInfo {
  enabled: boolean;
  port: number;
  token: string;
  /** 服务是否正在运行 */
  running: boolean;
  /** 实际监听端口（运行中才有值；与 port 不同说明配置端口被占用已回退） */
  activePort: number | null;
}

interface TauriMcpServerInfo {
  enabled: boolean;
  port: number;
  token: string;
  running: boolean;
  active_port: number | null;
}

function toFrontend(raw: TauriMcpServerInfo): McpServerInfo {
  return {
    enabled: raw.enabled,
    port: raw.port,
    token: raw.token,
    running: raw.running,
    activePort: raw.active_port,
  };
}

export async function getMcpServerInfo(): Promise<McpServerInfo> {
  return toFrontend(await invoke<TauriMcpServerInfo>('get_mcp_server_info'));
}

/** 更新设置并应用（启用→确保运行，关闭→停止，端口变更→重启） */
export async function updateMcpServerSettings(input: { enabled?: boolean; port?: number }): Promise<McpServerInfo> {
  return toFrontend(
    await invoke<TauriMcpServerInfo>('update_mcp_server_settings', {
      enabled: input.enabled ?? null,
      port: input.port ?? null,
    }),
  );
}

/** 重新生成访问令牌（运行中会重启服务） */
export async function regenerateMcpServerToken(): Promise<McpServerInfo> {
  return toFrontend(await invoke<TauriMcpServerInfo>('regenerate_mcp_server_token'));
}

/**
 * MCP 服务 JSON 导入解析（纯函数，可单测）。
 *
 * 兼容 Claude Desktop / Cursor 等客户端的 `mcpServers` 配置格式：
 * - `{"mcpServers": {"名字": {...}}}`（标准包装）
 * - `{"名字": {...}, ...}`（无包装的名→配置映射）
 * - `[{ name, command|url, ... }]`（带 name 的数组）
 *
 * 单个条目支持 command（stdio）或 url（streamable_http）；
 * args/env 按标准格式读取，其余字段（type、headers、cwd、disabled 等）忽略，
 * 其中 `disabled: true` 会映射为禁用。
 */

import type { McpTransportType } from '@/types/mcp-service';

export interface ParsedMcpServerConfig {
  name: string;
  transportType: McpTransportType;
  /** stdio 命令 */
  command: string;
  /** stdio 参数，保持数组形态，落库前再转按行文本 */
  args: string[];
  /** 环境变量键值对 */
  env: Record<string, string>;
  /** streamable_http 地址 */
  url: string;
  /** disabled: true 时为 false */
  enabled: boolean;
}

export interface McpJsonParseResult {
  services: ParsedMcpServerConfig[];
  /** 解析失败的条目及原因 */
  errors: string[];
}

interface RawServerConfig {
  command?: unknown;
  args?: unknown;
  env?: unknown;
  url?: unknown;
  name?: unknown;
  disabled?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === 'string').map(item => item.trim());
}

function asEnvRecord(value: unknown): Record<string, string> {
  if (!isRecord(value)) {
    return {};
  }
  const env: Record<string, string> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string') {
      env[key] = entry;
    } else if (typeof entry === 'number' || typeof entry === 'boolean') {
      env[key] = String(entry);
    }
  }
  return env;
}

function parseServerConfig(name: string, raw: RawServerConfig): ParsedMcpServerConfig | string {
  const trimmedName = name.trim() || asString(raw.name);
  if (!trimmedName) {
    return '条目缺少服务名';
  }

  const command = asString(raw.command);
  const url = asString(raw.url);

  if (command) {
    return {
      name: trimmedName,
      transportType: 'stdio',
      command,
      args: asStringArray(raw.args),
      env: asEnvRecord(raw.env),
      url: '',
      enabled: raw.disabled !== true,
    };
  }

  if (url) {
    return {
      name: trimmedName,
      transportType: 'streamable_http',
      command: '',
      args: [],
      env: asEnvRecord(raw.env),
      url,
      enabled: raw.disabled !== true,
    };
  }

  return '既没有 command 也没有 url，无法判断类型';
}

export function parseMcpServersJson(text: string): McpJsonParseResult {
  const trimmed = text.trim();
  if (!trimmed) {
    return { services: [], errors: ['请先粘贴 JSON 配置'] };
  }

  let value: unknown;
  try {
    value = JSON.parse(trimmed);
  } catch (error) {
    return { services: [], errors: [`不是合法的 JSON: ${error instanceof Error ? error.message : String(error)}`] };
  }

  const errors: string[] = [];
  const entries: Array<{ name: string; config: RawServerConfig }> = [];

  if (isRecord(value) && isRecord(value.mcpServers)) {
    for (const [name, config] of Object.entries(value.mcpServers)) {
      if (isRecord(config)) {
        entries.push({ name, config });
      } else {
        errors.push(`「${name}」的配置不是对象`);
      }
    }
  } else if (Array.isArray(value)) {
    for (const item of value) {
      if (isRecord(item)) {
        entries.push({ name: asString(item.name), config: item });
      } else {
        errors.push('数组里存在非对象的条目');
      }
    }
  } else if (isRecord(value)) {
    const values = Object.entries(value);
    if (values.length > 0 && values.every(([, config]) => isRecord(config))) {
      for (const [name, config] of values) {
        entries.push({ name, config: config as RawServerConfig });
      }
    } else {
      errors.push('无法识别的格式：期望 {"mcpServers": {...}}、名字到配置的映射，或带 name 的数组');
    }
  } else {
    errors.push('JSON 顶层应该是对象或数组');
  }

  const services: ParsedMcpServerConfig[] = [];
  for (const { name, config } of entries) {
    const parsed = parseServerConfig(name, config);
    if (typeof parsed === 'string') {
      errors.push(`「${name || '未命名'}」：${parsed}`);
    } else {
      services.push(parsed);
    }
  }

  if (entries.length > 0 && services.length === 0) {
    // 全部失败时补充总提示，避免用户只看到零散错误
    errors.unshift('没有解析出任何可导入的服务');
  }

  return { services, errors };
}

/** 数组 → 落库用的按行文本 */
export function argsToLines(args: string[]): string {
  return args.join('\n');
}

/** 环境变量对象 → KEY=value 按行文本 */
export function envToLines(env: Record<string, string>): string {
  return Object.entries(env)
    .map(([key, value]) => `${key}=${value}`)
    .join('\n');
}

import { describe, expect, it } from 'vitest';

import { argsToLines, envToLines, parseMcpServersJson } from '../parse-mcp-json';

describe('parseMcpServersJson', () => {
  it('parses the standard mcpServers wrapper', () => {
    const result = parseMcpServersJson(`{
      "mcpServers": {
        "fetch": { "command": "uvx", "args": ["mcp-server-fetch"], "env": { "KEY": "value" } },
        "eidolon": { "url": "http://127.0.0.1:9090/mcp", "headers": { "token": "x" } }
      }
    }`);

    expect(result.errors).toEqual([]);
    expect(result.services).toHaveLength(2);

    const fetchServer = result.services[0]!;
    expect(fetchServer.name).toBe('fetch');
    expect(fetchServer.transportType).toBe('stdio');
    expect(fetchServer.command).toBe('uvx');
    expect(fetchServer.args).toEqual(['mcp-server-fetch']);
    expect(fetchServer.env).toEqual({ KEY: 'value' });
    expect(fetchServer.enabled).toBe(true);

    const httpServer = result.services[1]!;
    expect(httpServer.transportType).toBe('streamable_http');
    expect(httpServer.url).toBe('http://127.0.0.1:9090/mcp');
  });

  it('parses a bare name-to-config map without the wrapper', () => {
    const result = parseMcpServersJson(`{
      "search": { "command": "npx", "args": ["-y", "mcp-search"] }
    }`);

    expect(result.errors).toEqual([]);
    expect(result.services[0]?.name).toBe('search');
    expect(result.services[0]?.args).toEqual(['-y', 'mcp-search']);
  });

  it('parses an array of named configs', () => {
    const result = parseMcpServersJson(
      `[{ "name": "a", "command": "node", "args": ["a.js"] }, { "name": "b", "url": "http://x/mcp" }]`,
    );

    expect(result.errors).toEqual([]);
    expect(result.services.map(service => service.name)).toEqual(['a', 'b']);
  });

  it('respects disabled and collects invalid entries as errors', () => {
    const result = parseMcpServersJson(`{
      "mcpServers": {
        "off": { "command": "node", "disabled": true },
        "broken": { "args": ["no-command"] },
        "not-an-object": "oops"
      }
    }`);

    expect(result.services).toHaveLength(1);
    expect(result.services[0]?.enabled).toBe(false);
    expect(result.errors).toHaveLength(2);
    expect(result.errors.some(error => error.includes('broken'))).toBe(true);
    expect(result.errors.some(error => error.includes('not-an-object'))).toBe(true);
  });

  it('reports a clear error when nothing parsed', () => {
    const result = parseMcpServersJson(`{ "mcpServers": { "broken": { "args": [] } } }`);
    expect(result.services).toHaveLength(0);
    expect(result.errors[0]).toContain('没有解析出任何可导入的服务');
  });

  it('rejects invalid json and empty input', () => {
    const bad = parseMcpServersJson('{ broken');
    expect(bad.services).toHaveLength(0);
    expect(bad.errors[0]).toContain('不是合法的 JSON');

    const empty = parseMcpServersJson('   ');
    expect(empty.services).toHaveLength(0);
    expect(empty.errors[0]).toBe('请先粘贴 JSON 配置');
  });

  it('converts args and env to persisted line format', () => {
    expect(argsToLines(['-y', 'mcp-search'])).toBe('-y\nmcp-search');
    expect(argsToLines([])).toBe('');
    expect(envToLines({ A: '1', B: '2' })).toBe('A=1\nB=2');
    expect(envToLines({})).toBe('');
  });
});

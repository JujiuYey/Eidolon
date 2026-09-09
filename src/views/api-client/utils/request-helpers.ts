import type {
  ApiClientBodyKind,
  ApiClientEnvironment,
  ApiClientKeyValueRow,
  ApiClientRequestBody,
  ApiClientRequestSnapshot,
} from '@/types/api-client';

export function createId(prefix = 'id'): string {
  const random = Math.random().toString(36).slice(2, 10);
  return `${prefix}_${Date.now().toString(36)}_${random}`;
}

export function createEmptyRow(): ApiClientKeyValueRow {
  return {
    id: createId('row'),
    enabled: true,
    key: '',
    value: '',
  };
}

export function createEmptyBody(kind: ApiClientBodyKind = 'none'): ApiClientRequestBody {
  return {
    kind,
    text: '',
    form: [],
  };
}

export function cloneRows(rows: ApiClientKeyValueRow[]): ApiClientKeyValueRow[] {
  return rows.map(row => ({ ...row }));
}

export function cloneBody(body: ApiClientRequestBody): ApiClientRequestBody {
  return {
    kind: body.kind,
    text: body.text,
    form: cloneRows(body.form),
  };
}

export interface ParsedFullUrl {
  baseUrl: string;
  query: Array<{ key: string; value: string }>;
}

const ABSOLUTE_URL_REGEX = /^[a-z][a-z0-9+.-]*:\/\//i;

export function isAbsoluteUrl(value: string): boolean {
  return ABSOLUTE_URL_REGEX.test(value.trim());
}

export function parseFullUrl(input: string): ParsedFullUrl | null {
  const trimmed = input.trim();
  if (!trimmed || !isAbsoluteUrl(trimmed)) {
    return null;
  }

  const questionIndex = trimmed.indexOf('?');
  if (questionIndex === -1) {
    return { baseUrl: trimmed, query: [] };
  }

  const baseUrl = trimmed.slice(0, questionIndex);
  const rawQuery = trimmed.slice(questionIndex + 1);

  if (!rawQuery) {
    return { baseUrl, query: [] };
  }

  const query: Array<{ key: string; value: string }> = [];
  const segments = rawQuery.split('&');
  for (const segment of segments) {
    if (!segment) {
      continue;
    }
    const equalIndex = segment.indexOf('=');
    if (equalIndex === -1) {
      query.push({ key: decodeQueryComponent(segment), value: '' });
    } else {
      const key = decodeQueryComponent(segment.slice(0, equalIndex));
      const value = decodeQueryComponent(segment.slice(equalIndex + 1));
      query.push({ key, value });
    }
  }

  return { baseUrl, query };
}

function decodeQueryComponent(input: string): string {
  try {
    return decodeURIComponent(input.replace(/\+/g, ' '));
  } catch {
    return input;
  }
}

export function splitUrlIntoBaseAndParams(input: string): {
  baseUrl: string;
  rows: ApiClientKeyValueRow[];
} {
  const parsed = parseFullUrl(input);
  if (!parsed) {
    return { baseUrl: input, rows: [] };
  }

  const rows: ApiClientKeyValueRow[] = parsed.query.map(entry => ({
    id: createId('row'),
    enabled: true,
    key: entry.key,
    value: entry.value,
  }));

  return { baseUrl: parsed.baseUrl, rows };
}

export function appendParamsToUrl(baseUrl: string, rows: ApiClientKeyValueRow[]): string {
  const trimmedBase = baseUrl.trim();
  if (!trimmedBase) {
    return trimmedBase;
  }

  const enabledPairs = rows
    .filter(row => row.enabled && row.key.trim() !== '')
    .map(row => `${encodeQueryComponent(row.key.trim())}=${encodeQueryComponent(row.value)}`);

  if (enabledPairs.length === 0) {
    return trimmedBase;
  }

  const separator = trimmedBase.includes('?') ? '&' : '?';
  return `${trimmedBase}${separator}${enabledPairs.join('&')}`;
}

function encodeQueryComponent(input: string): string {
  return encodeURIComponent(input);
}

export function buildRequestSnapshot(input: {
  method: ApiClientRequestSnapshot['method'];
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  timeoutMs: number;
  environmentName: string | null;
}): ApiClientRequestSnapshot {
  return {
    method: input.method,
    url: input.url,
    query: cloneRows(input.query),
    headers: cloneRows(input.headers),
    body: cloneBody(input.body),
    timeoutMs: input.timeoutMs,
    environmentName: input.environmentName,
  };
}

export function isRowEmpty(row: ApiClientKeyValueRow): boolean {
  return row.key.trim() === '' && row.value.trim() === '';
}

export function trimRows(rows: ApiClientKeyValueRow[]): ApiClientKeyValueRow[] {
  return rows.filter(row => !isRowEmpty(row));
}

export function hasEnabledHeader(rows: ApiClientKeyValueRow[]): boolean {
  return rows.some(row => row.enabled && row.key.trim() !== '');
}

const VARIABLE_PATTERN = /\{\{\s*([a-z_]\w*)\s*\}\}/gi;

export interface EnvironmentVariableLookup {
  resolve: (name: string) => string | undefined;
  isSensitive: (name: string) => boolean;
}

export interface VariableResolutionResult {
  text: string;
  missing: string[];
}

export function resolveVariables(text: string, lookup: EnvironmentVariableLookup): VariableResolutionResult {
  const missing = new Set<string>();

  const replaced = text.replace(VARIABLE_PATTERN, (match, name: string) => {
    const value = lookup.resolve(name);
    if (value === undefined) {
      missing.add(name);
      return match;
    }
    return value;
  });

  return { text: replaced, missing: Array.from(missing) };
}

export function maskSensitiveText(text: string, lookup: EnvironmentVariableLookup): string {
  return text.replace(VARIABLE_PATTERN, (match, name: string) => {
    return lookup.isSensitive(name) ? '••••' : match;
  });
}

const AUTH_HEADER_PATTERN = /^(authorization|proxy-authorization|x-api-key|api-key)$/i;

export function isSensitiveHeaderKey(key: string): boolean {
  return AUTH_HEADER_PATTERN.test(key.trim());
}

export function isSensitiveVariableName(name: string): boolean {
  const normalized = name.trim().toLowerCase();
  if (!normalized) {
    return false;
  }
  if (/(token|secret|password|passwd|api[-_]?key|access[-_]?key)/i.test(normalized)) {
    return true;
  }
  return false;
}

export function buildEnvironmentLookup(
  variables: ApiClientKeyValueRow[],
  defaultSensitive = false,
): EnvironmentVariableLookup {
  const map = new Map<string, { value: string; sensitive: boolean }>();
  for (const row of variables) {
    if (!row.enabled) {
      continue;
    }
    const name = row.key.trim();
    if (!name) {
      continue;
    }
    map.set(name, {
      value: row.value,
      sensitive: defaultSensitive || isSensitiveVariableName(name),
    });
  }

  return {
    resolve(name: string): string | undefined {
      const entry = map.get(name.trim());
      return entry?.value;
    },
    isSensitive(name: string): boolean {
      const entry = map.get(name.trim());
      if (entry) {
        return entry.sensitive;
      }
      return isSensitiveVariableName(name);
    },
  };
}

export interface BuildExecutionSnapshotOptions {
  baseUrl: string;
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  environment: { baseUrl: string; name: string | null } | null;
  lookup: EnvironmentVariableLookup | null;
}

export interface BuiltExecutionUrl {
  resolvedUrl: string;
  missingVariables: string[];
}

export function resolveExecutionUrl(options: BuildExecutionSnapshotOptions): BuiltExecutionUrl {
  const trimmedUrl = options.url.trim();
  const lookup = options.lookup;
  const resolveTarget = (target: string): { value: string; missing: string[] } => {
    if (!lookup) {
      return { value: target, missing: [] };
    }
    const result = resolveVariables(target, lookup);
    return { value: result.text, missing: result.missing };
  };

  const missing = new Set<string>();
  let resolvedTarget = trimmedUrl;

  if (trimmedUrl) {
    const urlResolution = resolveTarget(trimmedUrl);
    resolvedTarget = urlResolution.value;
    for (const name of urlResolution.missing) {
      missing.add(name);
    }
  }

  if (lookup && options.environment && !isAbsoluteUrl(resolvedTarget)) {
    const baseResolution = resolveTarget(options.environment.baseUrl.trim());
    for (const name of baseResolution.missing) {
      missing.add(name);
    }

    const baseTrimmed = baseResolution.value.replace(/\/+$/, '');
    if (baseTrimmed) {
      resolvedTarget = resolvedTarget.startsWith('/')
        ? `${baseTrimmed}${resolvedTarget}`
        : `${baseTrimmed}/${resolvedTarget}`;
    }
  }

  const queryMissing = lookup ? collectMissingVariablesInRows(options.query, lookup) : [];
  for (const name of queryMissing) {
    missing.add(name);
  }

  const finalUrl = appendParamsToUrl(resolvedTarget, options.query);
  return { resolvedUrl: finalUrl, missingVariables: Array.from(missing) };
}

export function collectMissingVariablesInRows(
  rows: ApiClientKeyValueRow[],
  lookup: EnvironmentVariableLookup,
): string[] {
  const missing = new Set<string>();
  for (const row of rows) {
    if (!row.enabled) {
      continue;
    }
    const keyResult = resolveVariables(row.key, lookup);
    const valueResult = resolveVariables(row.value, lookup);
    for (const name of keyResult.missing) {
      missing.add(name);
    }
    for (const name of valueResult.missing) {
      missing.add(name);
    }
  }
  return Array.from(missing);
}

export function collectMissingVariablesInBody(
  body: ApiClientRequestBody,
  lookup: EnvironmentVariableLookup,
): string[] {
  const missing = new Set<string>();
  const textResult = resolveVariables(body.text, lookup);
  for (const name of textResult.missing) {
    missing.add(name);
  }
  for (const name of collectMissingVariablesInRows(body.form, lookup)) {
    missing.add(name);
  }
  return Array.from(missing);
}

export function tryFormatJson(text: string): { ok: boolean; formatted: string | null; error: string | null } {
  if (!text.trim()) {
    return { ok: false, formatted: null, error: null };
  }

  try {
    const parsed = JSON.parse(text);
    return { ok: true, formatted: JSON.stringify(parsed, null, 2), error: null };
  } catch (error) {
    return {
      ok: false,
      formatted: null,
      error: error instanceof Error ? error.message : 'JSON 解析失败',
    };
  }
}

export interface DraftDiffInput {
  saved: ApiClientRequestSnapshot | null;
  savedName: string;
  draft: {
    name: string;
    method: ApiClientRequestSnapshot['method'];
    url: string;
    query: ApiClientKeyValueRow[];
    headers: ApiClientKeyValueRow[];
    body: ApiClientRequestBody;
    timeoutMs: number;
  };
}

export interface DraftDiffResult {
  isDirty: boolean;
  changedFields: string[];
}

export function diffDraftAgainstSaved(input: DraftDiffInput): DraftDiffResult {
  if (!input.saved) {
    return {
      isDirty: true,
      changedFields: ['name', 'method', 'url', 'query', 'headers', 'body', 'timeoutMs'],
    };
  }

  const changed: string[] = [];
  const draft = input.draft;
  const saved = input.saved;

  if (draft.name.trim() !== input.savedName.trim()) {
    changed.push('name');
  }

  if (draft.method !== saved.method) {
    changed.push('method');
  }

  if (draft.url !== saved.url) {
    changed.push('url');
  }

  if (!rowsEqual(draft.query, saved.query)) {
    changed.push('query');
  }

  if (!rowsEqual(draft.headers, saved.headers)) {
    changed.push('headers');
  }

  if (!bodyEqual(draft.body, saved.body)) {
    changed.push('body');
  }

  if (draft.timeoutMs !== saved.timeoutMs) {
    changed.push('timeoutMs');
  }

  return {
    isDirty: changed.length > 0,
    changedFields: changed,
  };
}

export function rowsEqual(a: ApiClientKeyValueRow[], b: ApiClientKeyValueRow[]): boolean {
  if (a.length !== b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i += 1) {
    const left = a[i];
    const right = b[i];
    if (!left || !right) {
      return false;
    }
    if (left.enabled !== right.enabled
      || left.key !== right.key
      || left.value !== right.value) {
      return false;
    }
  }
  return true;
}

export function bodyEqual(a: ApiClientRequestBody, b: ApiClientRequestBody): boolean {
  if (a.kind !== b.kind || a.text !== b.text) {
    return false;
  }
  return rowsEqual(a.form, b.form);
}

export function maskHeadersForHistory(
  rows: ApiClientKeyValueRow[],
): ApiClientKeyValueRow[] {
  return rows.map(row => {
    if (!row.enabled || !row.key.trim()) {
      return { ...row };
    }
    if (isSensitiveHeaderKey(row.key)) {
      return { ...row, value: '••••' };
    }
    return { ...row };
  });
}

export function maskQueryForHistory(rows: ApiClientKeyValueRow[]): ApiClientKeyValueRow[] {
  return cloneRows(rows);
}

export interface ExecutionPreparationInput {
  url: string;
  query: ApiClientKeyValueRow[];
  headers: ApiClientKeyValueRow[];
  body: ApiClientRequestBody;
  method: ApiClientRequestSnapshot['method'];
  timeoutMs: number;
  environment: ApiClientEnvironment | null;
}

export interface ExecutionPreparation {
  snapshot: ApiClientRequestSnapshot;
  missingVariables: string[];
  environmentId: string | null;
  environmentName: string | null;
}

/**
 * 构造执行快照并集中校验未定义变量。Store 在调用 `send_api_request` 之前
 * 必须先得到一份 snapshot 与缺失变量列表；本函数是纯函数，便于在测试里直接覆盖。
 */
export function prepareExecution(input: ExecutionPreparationInput): ExecutionPreparation {
  const env = input.environment;
  const lookup = env ? buildEnvironmentLookup(env.variables, false) : null;
  const urlResolution = resolveExecutionUrl({
    baseUrl: env?.baseUrl ?? '',
    url: input.url,
    query: input.query,
    headers: input.headers,
    body: input.body,
    environment: env ? { baseUrl: env.baseUrl, name: env.name } : null,
    lookup,
  });

  const headerMissing = lookup ? collectMissingVariablesInRows(input.headers, lookup) : [];
  const bodyMissing = lookup ? collectMissingVariablesInBody(input.body, lookup) : [];
  const missingVariables = Array.from(new Set([
    ...urlResolution.missingVariables,
    ...headerMissing,
    ...bodyMissing,
  ]));

  return {
    snapshot: buildRequestSnapshot({
      method: input.method,
      url: input.url,
      query: input.query,
      headers: input.headers,
      body: input.body,
      timeoutMs: input.timeoutMs,
      environmentName: env?.name ?? null,
    }),
    missingVariables,
    environmentId: env?.id ?? null,
    environmentName: env?.name ?? null,
  };
}

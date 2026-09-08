import { strict as assert } from 'node:assert';
import { describe, it } from 'vitest';
import {
  appendParamsToUrl,
  buildEnvironmentLookup,
  buildRequestSnapshot,
  cloneBody,
  cloneRows,
  collectMissingVariablesInBody,
  collectMissingVariablesInRows,
  createEmptyBody,
  createEmptyRow,
  diffDraftAgainstSaved,
  isAbsoluteUrl,
  isSensitiveHeaderKey,
  isSensitiveVariableName,
  maskHeadersForHistory,
  maskSensitiveText,
  parseFullUrl,
  resolveExecutionUrl,
  resolveVariables,
  rowsEqual,
  splitUrlIntoBaseAndParams,
  trimRows,
  tryFormatJson,
} from '@/views/api-client/utils/request-helpers';
import type { ApiClientKeyValueRow, ApiClientRequestBody } from '@/types/api-client';

function makeRow(overrides: Partial<ApiClientKeyValueRow> = {}): ApiClientKeyValueRow {
  return {
    id: 'row-test',
    enabled: true,
    key: '',
    value: '',
    ...overrides,
  };
}

describe('parseFullUrl + splitUrlIntoBaseAndParams', () => {
  it('parses absolute URL without query string', () => {
    const parsed = parseFullUrl('https://example.com/api/v1/users');
    assert.deepEqual(parsed, { baseUrl: 'https://example.com/api/v1/users', query: [] });
  });

  it('parses absolute URL with single query parameter', () => {
    const parsed = parseFullUrl('https://example.com/api?foo=bar');
    assert.deepEqual(parsed, {
      baseUrl: 'https://example.com/api',
      query: [{ key: 'foo', value: 'bar' }],
    });
  });

  it('parses absolute URL with repeated query keys preserving order', () => {
    const parsed = parseFullUrl('https://example.com/api?tag=a&tag=b&tag=c');
    assert.equal(parsed?.baseUrl, 'https://example.com/api');
    assert.deepEqual(parsed?.query, [
      { key: 'tag', value: 'a' },
      { key: 'tag', value: 'b' },
      { key: 'tag', value: 'c' },
    ]);
  });

  it('parses query with no value', () => {
    const parsed = parseFullUrl('https://example.com/api?flag');
    assert.deepEqual(parsed?.query, [{ key: 'flag', value: '' }]);
  });

  it('decodes URL-encoded values', () => {
    const parsed = parseFullUrl('https://example.com/api?q=hello%20world&name=%E4%B8%AD%E6%96%87');
    assert.equal(parsed?.query[0]?.value, 'hello world');
    assert.equal(parsed?.query[1]?.value, '中文');
  });

  it('returns null for non-absolute input', () => {
    assert.equal(parseFullUrl('/api/v1/users'), null);
    assert.equal(parseFullUrl(''), null);
  });

  it('splitUrlIntoBaseAndParams returns rows with default enabled=true', () => {
    const { baseUrl, rows } = splitUrlIntoBaseAndParams('https://example.com/api?a=1&a=2&b=3');
    assert.equal(baseUrl, 'https://example.com/api');
    assert.equal(rows.length, 3);
    for (const row of rows) {
      assert.equal(row.enabled, true);
      assert.notEqual(row.id, '');
    }
    assert.deepEqual(rows.map(r => [r.key, r.value]), [['a', '1'], ['a', '2'], ['b', '3']]);
  });

  it('isAbsoluteUrl detects only scheme-prefixed URLs', () => {
    assert.equal(isAbsoluteUrl('https://example.com'), true);
    assert.equal(isAbsoluteUrl('http://example.com'), true);
    assert.equal(isAbsoluteUrl('ws://example.com'), true);
    assert.equal(isAbsoluteUrl('//example.com'), false);
    assert.equal(isAbsoluteUrl('/relative'), false);
  });
});

describe('appendParamsToUrl', () => {
  it('appends enabled rows to a base URL', () => {
    const url = appendParamsToUrl('https://example.com/api', [
      makeRow({ key: 'a', value: '1' }),
      makeRow({ key: 'b', value: 'two words' }),
    ]);
    assert.equal(url, 'https://example.com/api?a=1&b=two%20words');
  });

  it('preserves existing query string in base URL', () => {
    const url = appendParamsToUrl('https://example.com/api?z=0', [
      makeRow({ key: 'a', value: '1' }),
    ]);
    assert.equal(url, 'https://example.com/api?z=0&a=1');
  });

  it('skips disabled rows and empty keys', () => {
    const url = appendParamsToUrl('https://example.com/api', [
      makeRow({ enabled: false, key: 'a', value: '1' }),
      makeRow({ key: '', value: 'ignored' }),
      makeRow({ key: 'b', value: 'two' }),
    ]);
    assert.equal(url, 'https://example.com/api?b=two');
  });
});

describe('rowsEqual and cloneRows', () => {
  it('detects order and value differences', () => {
    const a = [makeRow({ key: 'a', value: '1' }), makeRow({ key: 'b', value: '2' })];
    const b = [makeRow({ key: 'a', value: '1' }), makeRow({ key: 'b', value: '2' })];
    const c = [makeRow({ key: 'b', value: '2' }), makeRow({ key: 'a', value: '1' })];
    assert.equal(rowsEqual(a, b), true);
    assert.equal(rowsEqual(a, c), false);
  });

  it('cloneRows returns independent rows', () => {
    const source = [makeRow({ key: 'a' })];
    const cloned = cloneRows(source);
    cloned[0]!.key = 'changed';
    assert.equal(source[0]!.key, 'a');
  });

  it('trimRows drops rows where both key and value are empty', () => {
    const trimmed = trimRows([
      makeRow({ key: '', value: '' }),
      makeRow({ key: 'a' }),
      makeRow({ key: 'only-value', value: '' }),
    ]);
    assert.equal(trimmed.length, 2);
    assert.equal(trimmed[0]!.key, 'a');
    assert.equal(trimmed[1]!.key, 'only-value');
  });
});

describe('createEmptyRow / createEmptyBody / cloneBody', () => {
  it('createEmptyRow creates an enabled empty row with id', () => {
    const row = createEmptyRow();
    assert.equal(row.enabled, true);
    assert.equal(row.key, '');
    assert.equal(row.value, '');
    assert.notEqual(row.id, '');
  });

  it('createEmptyBody defaults to none kind', () => {
    const body = createEmptyBody();
    assert.equal(body.kind, 'none');
    assert.equal(body.text, '');
    assert.deepEqual(body.form, []);
  });

  it('cloneBody is independent of original', () => {
    const original: ApiClientRequestBody = createEmptyBody('json');
    original.text = '{"a":1}';
    original.form = [makeRow({ key: 'a', value: '1' })];
    const cloned = cloneBody(original);
    cloned.text = 'changed';
    cloned.form[0]!.value = 'changed';
    assert.equal(original.text, '{"a":1}');
    assert.equal(original.form[0]!.value, '1');
  });
});

describe('tryFormatJson', () => {
  it('returns ok=true with formatted JSON for valid input', () => {
    const result = tryFormatJson('{"a":1,"b":[1,2]}');
    assert.equal(result.ok, true);
    assert.equal(result.error, null);
    assert.equal(result.formatted, '{\n  "a": 1,\n  "b": [\n    1,\n    2\n  ]\n}');
  });

  it('returns error for invalid JSON', () => {
    const result = tryFormatJson('{invalid');
    assert.equal(result.ok, false);
    assert.notEqual(result.error, null);
    assert.equal(result.formatted, null);
  });

  it('treats empty string as not-an-error', () => {
    const result = tryFormatJson('');
    assert.equal(result.ok, false);
    assert.equal(result.formatted, null);
    assert.equal(result.error, null);
  });
});

describe('resolveVariables', () => {
  const lookup = buildEnvironmentLookup(
    [
      makeRow({ key: 'baseUrl', value: 'https://api.example.com' }),
      makeRow({ key: 'token', value: 'secret' }),
    ],
    false,
  );

  it('replaces variables when present', () => {
    const result = resolveVariables('{{baseUrl}}/users?token={{token}}', lookup);
    assert.equal(result.text, 'https://api.example.com/users?token=secret');
    assert.deepEqual(result.missing, []);
  });

  it('reports missing variables without throwing', () => {
    const result = resolveVariables('{{baseUrl}}/{{unknown}}', lookup);
    assert.equal(result.text, 'https://api.example.com/{{unknown}}');
    assert.deepEqual(result.missing.sort(), ['unknown']);
  });
});

describe('buildEnvironmentLookup sensitivity', () => {
  it('marks token-like names as sensitive', () => {
    const lookup = buildEnvironmentLookup([
      makeRow({ key: 'API_KEY', value: 'secret' }),
      makeRow({ key: 'plain_value', value: 'visible' }),
    ]);
    assert.equal(lookup.isSensitive('API_KEY'), true);
    assert.equal(lookup.isSensitive('plain_value'), false);
  });

  it('isSensitiveVariableName identifies common sensitive names', () => {
    assert.equal(isSensitiveVariableName('auth_token'), true);
    assert.equal(isSensitiveVariableName('api_key'), true);
    assert.equal(isSensitiveVariableName('password'), true);
    assert.equal(isSensitiveVariableName('trace_id'), false);
  });
});

describe('maskSensitiveText', () => {
  it('masks sensitive variables and keeps others intact', () => {
    const lookup = buildEnvironmentLookup([
      makeRow({ key: 'TOKEN', value: 'secret' }),
      makeRow({ key: 'host', value: 'visible' }),
    ]);
    const masked = maskSensitiveText('Bearer {{TOKEN}} on {{host}}', lookup);
    assert.equal(masked, 'Bearer •••• on {{host}}');
  });
});

describe('isSensitiveHeaderKey', () => {
  it('matches common auth headers case-insensitively', () => {
    assert.equal(isSensitiveHeaderKey('Authorization'), true);
    assert.equal(isSensitiveHeaderKey('authorization'), true);
    assert.equal(isSensitiveHeaderKey('X-API-KEY'), true);
    assert.equal(isSensitiveHeaderKey('api-key'), true);
    assert.equal(isSensitiveHeaderKey('proxy-authorization'), true);
    assert.equal(isSensitiveHeaderKey('Content-Type'), false);
  });
});

describe('maskHeadersForHistory', () => {
  it('redacts sensitive header values', () => {
    const headers: ApiClientKeyValueRow[] = [
      makeRow({ key: 'Authorization', value: 'Bearer xyz' }),
      makeRow({ key: 'Content-Type', value: 'application/json' }),
    ];
    const masked = maskHeadersForHistory(headers);
    assert.equal(masked[0]!.value, '••••');
    assert.equal(masked[1]!.value, 'application/json');
  });

  it('does not redact disabled or empty-key rows', () => {
    const headers: ApiClientKeyValueRow[] = [
      makeRow({ enabled: false, key: 'Authorization', value: 'Bearer xyz' }),
      makeRow({ key: '', value: 'should be untouched' }),
    ];
    const masked = maskHeadersForHistory(headers);
    assert.equal(masked[0]!.value, 'Bearer xyz');
    assert.equal(masked[1]!.value, 'should be untouched');
  });
});

describe('collectMissingVariablesInRows', () => {
  it('returns unique missing variables from enabled rows', () => {
    const lookup = buildEnvironmentLookup([makeRow({ key: 'present', value: 'x' })]);
    const rows: ApiClientKeyValueRow[] = [
      makeRow({ key: '{{a}}', value: '1' }),
      makeRow({ key: '{{a}}', value: '2' }),
      makeRow({ enabled: false, key: '{{a}}', value: 'ignored' }),
      makeRow({ key: 'present', value: 'ok' }),
    ];
    const missing = collectMissingVariablesInRows(rows, lookup);
    assert.deepEqual(missing, ['a']);
  });
});

describe('collectMissingVariablesInBody', () => {
  it('collects missing variables in JSON text and form rows', () => {
    const lookup = buildEnvironmentLookup([makeRow({ key: 'host', value: 'h' })]);
    const body: ApiClientRequestBody = {
      kind: 'json',
      text: '{"url":"{{host}}/{{path}}"}',
      form: [makeRow({ key: 'a', value: '{{path}}' })],
    };
    const missing = collectMissingVariablesInBody(body, lookup);
    assert.deepEqual(missing.sort(), ['path']);
  });
});

describe('resolveExecutionUrl', () => {
  it('resolves absolute URL with variables and missing variable detection', () => {
    const lookup = buildEnvironmentLookup([makeRow({ key: 'host', value: 'https://api.example.com' })]);
    const result = resolveExecutionUrl({
      baseUrl: '',
      url: '{{host}}/users/{{missing}}',
      query: [],
      headers: [],
      body: createEmptyBody('none'),
      environment: { baseUrl: '{{host}}', name: 'dev' },
      lookup,
    });
    assert.equal(result.resolvedUrl, 'https://api.example.com/users/{{missing}}');
    assert.deepEqual(result.missingVariables.sort(), ['missing']);
  });

  it('uses environment baseUrl for relative URLs', () => {
    const lookup = buildEnvironmentLookup([]);
    const result = resolveExecutionUrl({
      baseUrl: 'https://api.example.com',
      url: '/users',
      query: [],
      headers: [],
      body: createEmptyBody('none'),
      environment: { baseUrl: 'https://api.example.com', name: 'dev' },
      lookup,
    });
    assert.equal(result.resolvedUrl, 'https://api.example.com/users');
    assert.deepEqual(result.missingVariables, []);
  });

  it('appends enabled query parameters in order', () => {
    const lookup = buildEnvironmentLookup([]);
    const result = resolveExecutionUrl({
      baseUrl: '',
      url: 'https://api.example.com/users',
      query: [
        makeRow({ key: 'a', value: '1' }),
        makeRow({ enabled: false, key: 'b', value: '2' }),
        makeRow({ key: 'a', value: '3' }),
      ],
      headers: [],
      body: createEmptyBody('none'),
      environment: null,
      lookup,
    });
    assert.equal(result.resolvedUrl, 'https://api.example.com/users?a=1&a=3');
  });
});

describe('buildRequestSnapshot', () => {
  it('produces an independent snapshot', () => {
    const row = makeRow({ key: 'a', value: '1' });
    const snapshot = buildRequestSnapshot({
      method: 'GET',
      url: 'https://example.com',
      query: [row],
      headers: [],
      body: createEmptyBody('json'),
      timeoutMs: 5000,
      environmentName: 'dev',
    });
    snapshot.query[0]!.value = 'changed';
    assert.equal(row.value, '1');
    assert.equal(snapshot.environmentName, 'dev');
  });
});

describe('diffDraftAgainstSaved', () => {
  it('returns dirty when there is no saved snapshot', () => {
    const result = diffDraftAgainstSaved({
      saved: null,
      savedName: '',
      draft: {
        name: 'a',
        method: 'GET',
        url: '',
        query: [],
        headers: [],
        body: createEmptyBody('none'),
        timeoutMs: 5000,
      },
    });
    assert.equal(result.isDirty, true);
  });

  it('returns not-dirty when draft matches saved snapshot', () => {
    const snapshot = buildRequestSnapshot({
      method: 'GET',
      url: 'https://example.com',
      query: [],
      headers: [],
      body: createEmptyBody('none'),
      timeoutMs: 5000,
      environmentName: 'dev',
    });
    const result = diffDraftAgainstSaved({
      saved: snapshot,
      savedName: 'a',
      draft: {
        name: 'a',
        method: 'GET',
        url: 'https://example.com',
        query: [],
        headers: [],
        body: createEmptyBody('none'),
        timeoutMs: 5000,
      },
    });
    assert.equal(result.isDirty, false);
    assert.deepEqual(result.changedFields, []);
  });

  it('detects method change only when name unchanged', () => {
    const snapshot = buildRequestSnapshot({
      method: 'GET',
      url: 'https://example.com',
      query: [],
      headers: [],
      body: createEmptyBody('none'),
      timeoutMs: 5000,
      environmentName: null,
    });
    const result = diffDraftAgainstSaved({
      saved: snapshot,
      savedName: 'a',
      draft: {
        name: 'a',
        method: 'POST',
        url: 'https://example.com',
        query: [],
        headers: [],
        body: createEmptyBody('none'),
        timeoutMs: 5000,
      },
    });
    assert.equal(result.isDirty, true);
    assert.deepEqual(result.changedFields, ['method']);
  });

  it('detects name change only', () => {
    const snapshot = buildRequestSnapshot({
      method: 'GET',
      url: 'https://example.com',
      query: [],
      headers: [],
      body: createEmptyBody('none'),
      timeoutMs: 5000,
      environmentName: null,
    });
    const result = diffDraftAgainstSaved({
      saved: snapshot,
      savedName: 'old-name',
      draft: {
        name: 'new-name',
        method: 'GET',
        url: 'https://example.com',
        query: [],
        headers: [],
        body: createEmptyBody('none'),
        timeoutMs: 5000,
      },
    });
    assert.equal(result.isDirty, true);
    assert.deepEqual(result.changedFields, ['name']);
  });

  it('detects query/headers/body/timeout changes', () => {
    const snapshot = buildRequestSnapshot({
      method: 'GET',
      url: 'https://example.com',
      query: [makeRow({ key: 'a', value: '1' })],
      headers: [makeRow({ key: 'X-Test', value: '1' })],
      body: createEmptyBody('json'),
      timeoutMs: 5000,
      environmentName: null,
    });
    const result = diffDraftAgainstSaved({
      saved: snapshot,
      savedName: 'a',
      draft: {
        name: 'a',
        method: 'GET',
        url: 'https://example.com',
        query: [makeRow({ key: 'a', value: '2' })],
        headers: [makeRow({ key: 'X-Test', value: '1' })],
        body: createEmptyBody('text'),
        timeoutMs: 7000,
      },
    });
    assert.equal(result.isDirty, true);
    assert.ok(result.changedFields.includes('query'));
    assert.ok(result.changedFields.includes('body'));
    assert.ok(result.changedFields.includes('timeoutMs'));
  });
});

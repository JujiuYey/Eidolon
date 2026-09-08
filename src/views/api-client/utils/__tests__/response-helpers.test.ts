import { strict as assert } from 'node:assert';
import { describe, it } from 'vitest';
import {
  buildResponseView,
  classifyExecutionStatus,
  classifyResponseKind,
} from '@/views/api-client/utils/response-helpers';

describe('classifyResponseKind', () => {
  it('cancelled wins over status code', () => {
    assert.equal(classifyResponseKind({
      cancelled: true,
      timedOut: false,
      connectionFailed: false,
      statusCode: 200,
      isOversized: false,
      isBinary: false,
    }), 'cancelled');
  });

  it('timeout wins over success status', () => {
    assert.equal(classifyResponseKind({
      cancelled: false,
      timedOut: true,
      connectionFailed: false,
      statusCode: 200,
      isOversized: false,
      isBinary: false,
    }), 'timeout');
  });

  it('network_error when connection fails', () => {
    assert.equal(classifyResponseKind({
      cancelled: false,
      timedOut: false,
      connectionFailed: true,
      statusCode: null,
      isOversized: false,
      isBinary: false,
    }), 'network_error');
  });

  it('success for 2xx/3xx, http_error for 4xx/5xx', () => {
    for (const status of [200, 201, 204, 301, 302, 399]) {
      assert.equal(classifyResponseKind({
        cancelled: false,
        timedOut: false,
        connectionFailed: false,
        statusCode: status,
        isOversized: false,
        isBinary: false,
      }), 'success');
    }
    for (const status of [400, 401, 404, 500, 502]) {
      assert.equal(classifyResponseKind({
        cancelled: false,
        timedOut: false,
        connectionFailed: false,
        statusCode: status,
        isOversized: false,
        isBinary: false,
      }), 'http_error');
    }
  });

  it('oversize / binary are distinct categories', () => {
    assert.equal(classifyResponseKind({
      cancelled: false,
      timedOut: false,
      connectionFailed: false,
      statusCode: 200,
      isOversized: true,
      isBinary: false,
    }), 'oversize');
    assert.equal(classifyResponseKind({
      cancelled: false,
      timedOut: false,
      connectionFailed: false,
      statusCode: 200,
      isOversized: false,
      isBinary: true,
    }), 'binary');
  });

  it('idle when status code is null', () => {
    assert.equal(classifyResponseKind({
      cancelled: false,
      timedOut: false,
      connectionFailed: false,
      statusCode: null,
      isOversized: false,
      isBinary: false,
    }), 'idle');
  });
});

describe('classifyExecutionStatus (history)', () => {
  it('cancelled maps to cancelled', () => {
    assert.equal(classifyExecutionStatus({
      ok: true,
      statusCode: 200,
      cancelled: true,
      timedOut: false,
      connectionFailed: false,
    }), 'cancelled');
  });

  it('timed-out maps to timeout', () => {
    assert.equal(classifyExecutionStatus({
      ok: false,
      statusCode: null,
      cancelled: false,
      timedOut: true,
      connectionFailed: false,
    }), 'timeout');
  });

  it('connection failure maps to network_error', () => {
    assert.equal(classifyExecutionStatus({
      ok: false,
      statusCode: null,
      cancelled: false,
      timedOut: false,
      connectionFailed: true,
    }), 'network_error');
  });

  it('4xx maps to http_error', () => {
    assert.equal(classifyExecutionStatus({
      ok: true,
      statusCode: 404,
      cancelled: false,
      timedOut: false,
      connectionFailed: false,
    }), 'http_error');
  });

  it('2xx maps to success', () => {
    assert.equal(classifyExecutionStatus({
      ok: true,
      statusCode: 200,
      cancelled: false,
      timedOut: false,
      connectionFailed: false,
    }), 'success');
  });
});

describe('buildResponseView', () => {
  it('attempts to format raw text when success', () => {
    const view = buildResponseView({
      kind: 'success',
      rawText: '{"a":1}',
      meta: { status: 200, durationMs: 100, sizeBytes: 7, contentType: 'application/json', headers: [] },
      errorMessage: null,
    });
    assert.equal(view.formattedJson, '{\n  "a": 1\n}');
    assert.equal(view.kind, 'success');
    assert.equal(view.meta?.status, 200);
  });

  it('keeps raw text and skips JSON formatting when kind is not success/http_error', () => {
    const view = buildResponseView({
      kind: 'network_error',
      rawText: '',
      meta: null,
      errorMessage: 'failed',
    });
    assert.equal(view.formattedJson, null);
    assert.equal(view.kind, 'network_error');
    assert.equal(view.errorMessage, 'failed');
  });

  it('preserves oversize/binary flags', () => {
    const view = buildResponseView({
      kind: 'oversize',
      rawText: '',
      meta: null,
      errorMessage: null,
      isOversized: true,
    });
    assert.equal(view.isOversized, true);
  });
});

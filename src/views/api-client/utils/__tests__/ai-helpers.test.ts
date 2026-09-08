import { strict as assert } from 'node:assert';
import { describe, it } from 'vitest';
import {
  applyCandidateToBody,
  buildAiCandidate,
  buildAiCandidateFailed,
  buildAiCandidateGenerating,
  buildAiContextPayload,
} from '@/views/api-client/utils/ai-helpers';
import { createEmptyBody } from '@/views/api-client/utils/request-helpers';
import type { ApiClientAiCandidate } from '@/types/api-client';

function baseInputs() {
  return {
    requestId: 'req-1',
    taskId: 'task-1',
    prompt: '生成订单',
    reference: '字段: userId, items',
    includeCurrentBody: false,
    modelLabel: 'mock-model',
  };
}

describe('buildAiCandidate', () => {
  it('marks valid JSON and clears error', () => {
    const candidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '{"orderId":"o-1"}',
    });
    assert.equal(candidate.isJsonValid, true);
    assert.equal(candidate.jsonError, null);
    assert.equal(candidate.status, 'success');
    assert.equal(candidate.content, '{"orderId":"o-1"}');
  });

  it('marks invalid JSON with error', () => {
    const candidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '{broken',
    });
    assert.equal(candidate.isJsonValid, false);
    assert.notEqual(candidate.jsonError, null);
    assert.equal(candidate.status, 'success');
  });

  it('allows valid JSON arrays and primitives', () => {
    const arrayCandidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '[1,2,3]',
    });
    assert.equal(arrayCandidate.isJsonValid, true);
    const primitiveCandidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '42',
    });
    assert.equal(primitiveCandidate.isJsonValid, true);
  });
});

describe('buildAiCandidateGenerating', () => {
  it('starts in generating status with empty content', () => {
    const candidate: ApiClientAiCandidate = buildAiCandidateGenerating(baseInputs());
    assert.equal(candidate.status, 'generating');
    assert.equal(candidate.content, '');
    assert.equal(candidate.isJsonValid, false);
  });
});

describe('buildAiCandidateFailed', () => {
  it('sets status to error with message', () => {
    const candidate = buildAiCandidateFailed({
      ...baseInputs(),
      errorMessage: '模型不可用',
    });
    assert.equal(candidate.status, 'error');
    assert.equal(candidate.errorMessage, '模型不可用');
  });
});

describe('applyCandidateToBody', () => {
  it('replaces entire body when current body is not JSON', () => {
    const candidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '{"a":1}',
    });
    const next = applyCandidateToBody(createEmptyBody('text'), candidate);
    assert.equal(next.kind, 'json');
    assert.equal(next.text, '{"a":1}');
    assert.deepEqual(next.form, []);
  });

  it('overwrites json text', () => {
    const candidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '{"new":true}',
    });
    const next = applyCandidateToBody({
      kind: 'json',
      text: '{"old":true}',
      form: [],
    }, candidate);
    assert.equal(next.kind, 'json');
    assert.equal(next.text, '{"new":true}');
  });

  it('refuses to apply invalid JSON', () => {
    const candidate = buildAiCandidate({
      ...baseInputs(),
      rawContent: '{broken',
    });
    const original = createEmptyBody('json');
    const next = applyCandidateToBody(original, candidate);
    assert.deepEqual(next, original);
  });
});

describe('buildAiContextPayload', () => {
  it('includes current body only when includeCurrentBody is true and body is JSON', () => {
    const payload = buildAiContextPayload({
      prompt: 'p',
      reference: 'r',
      includeCurrentBody: true,
      currentBody: { kind: 'json', text: '{"x":1}', form: [] },
      requestName: 'create order',
      method: 'POST',
      url: '/orders',
    });
    assert.equal(payload.hasCurrentBody, true);
    assert.equal(payload.currentBodyJson, '{"x":1}');
  });

  it('omits current body when kind is not JSON', () => {
    const payload = buildAiContextPayload({
      prompt: 'p',
      reference: 'r',
      includeCurrentBody: true,
      currentBody: { kind: 'text', text: 'hello', form: [] },
      requestName: 'create order',
      method: 'POST',
      url: '/orders',
    });
    assert.equal(payload.hasCurrentBody, false);
    assert.equal(payload.currentBodyJson, null);
  });

  it('omits current body when includeCurrentBody is false', () => {
    const payload = buildAiContextPayload({
      prompt: 'p',
      reference: 'r',
      includeCurrentBody: false,
      currentBody: { kind: 'json', text: '{"x":1}', form: [] },
      requestName: 'create order',
      method: 'POST',
      url: '/orders',
    });
    assert.equal(payload.hasCurrentBody, false);
  });
});

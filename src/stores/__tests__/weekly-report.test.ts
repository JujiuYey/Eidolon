import { describe, expect, it } from 'vitest';

import { normalizeAuthorMode } from '../weekly-report';

describe('normalizeAuthorMode', () => {
  it('keeps current modes as-is', () => {
    expect(normalizeAuthorMode('mine')).toBe('mine');
    expect(normalizeAuthorMode('custom')).toBe('custom');
    expect(normalizeAuthorMode('all')).toBe('all');
  });

  it('migrates legacy auto mode to mine', () => {
    expect(normalizeAuthorMode('auto')).toBe('mine');
  });

  it('falls back to mine for unknown values', () => {
    expect(normalizeAuthorMode('nonsense')).toBe('mine');
    expect(normalizeAuthorMode(undefined)).toBe('mine');
    expect(normalizeAuthorMode(null)).toBe('mine');
  });
});

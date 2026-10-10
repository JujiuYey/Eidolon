import { describe, expect, it } from 'vitest';

import { isMenuItemActive } from '../utils';

describe('isMenuItemActive', () => {
  const segmentItem = { key: 'mail', path: '/mail' };
  const exactItem = { key: 'api-client', path: '/api-client', exact: true };
  const projectItem = { key: 'api-project-1', path: '/api-client/projects/1', projectId: '1' };

  it('matches by first path segment by default', () => {
    expect(isMenuItemActive(segmentItem, { path: '/mail' })).toBe(true);
    expect(isMenuItemActive(segmentItem, { path: '/mail/compose' })).toBe(true);
    expect(isMenuItemActive(segmentItem, { path: '/docs' })).toBe(false);
  });

  it('treats root path as the index segment', () => {
    expect(isMenuItemActive(segmentItem, { path: '/' })).toBe(false);
  });

  it('matches exact paths only when exact is set', () => {
    expect(isMenuItemActive(exactItem, { path: '/api-client' })).toBe(true);
    expect(isMenuItemActive(exactItem, { path: '/api-client/projects/1' })).toBe(false);
  });

  it('matches project items by route name and params id', () => {
    expect(
      isMenuItemActive(projectItem, {
        path: '/api-client/projects/1',
        name: 'api-client-project',
        params: { id: '1' },
      }),
    ).toBe(true);
    expect(
      isMenuItemActive(projectItem, {
        path: '/api-client/projects/2',
        name: 'api-client-project',
        params: { id: '2' },
      }),
    ).toBe(false);
    expect(
      isMenuItemActive(projectItem, { path: '/api-client/projects/1', name: 'other-route', params: { id: '1' } }),
    ).toBe(false);
    expect(isMenuItemActive(projectItem, { path: '/api-client' })).toBe(false);
  });
});

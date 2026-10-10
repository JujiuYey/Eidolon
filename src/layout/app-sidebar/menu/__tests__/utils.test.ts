import { describe, expect, it } from 'vitest';

import { isMenuItemActive } from '../utils';

describe('isMenuItemActive', () => {
  const segmentItem = { key: 'mail', path: '/mail' };
  const exactItem = { key: 'api-client', path: '/api-client', exact: true };
  const routeMatchItem = {
    key: 'api-project-1',
    path: '/api-client/projects/1',
    routeMatch: { name: 'api-client-project', paramKey: 'id', paramValue: '1' },
  };

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

  it('matches sub-routes by route name and params key', () => {
    expect(
      isMenuItemActive(routeMatchItem, {
        path: '/api-client/projects/1',
        name: 'api-client-project',
        params: { id: '1' },
      }),
    ).toBe(true);
    expect(
      isMenuItemActive(routeMatchItem, {
        path: '/api-client/projects/2',
        name: 'api-client-project',
        params: { id: '2' },
      }),
    ).toBe(false);
    expect(
      isMenuItemActive(routeMatchItem, { path: '/api-client/projects/1', name: 'other-route', params: { id: '1' } }),
    ).toBe(false);
    expect(isMenuItemActive(routeMatchItem, { path: '/api-client' })).toBe(false);
  });

  it('matches docs roots by route name and rootId param', () => {
    const docsRootItem = {
      key: 'docs-root-abc',
      path: '/docs/r/abc',
      routeMatch: { name: 'docs-root', paramKey: 'rootId', paramValue: 'abc' },
    };
    expect(isMenuItemActive(docsRootItem, { path: '/docs/r/abc', name: 'docs-root', params: { rootId: 'abc' } })).toBe(
      true,
    );
    expect(isMenuItemActive(docsRootItem, { path: '/docs/r/xyz', name: 'docs-root', params: { rootId: 'xyz' } })).toBe(
      false,
    );
    expect(isMenuItemActive(docsRootItem, { path: '/docs' })).toBe(false);
  });
});

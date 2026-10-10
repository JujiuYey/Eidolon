export const API_PROJECT_ROUTE_NAME = 'api-client-project';

export interface MenuItemRouteLike {
  path: string;
  name?: unknown;
  params?: { id?: string | string[] };
}

export interface MenuItemMatchTarget {
  key: string;
  path: string;
  /** 按完整 path 精确匹配（固定入口，如 /api-client），避免子路由误亮 */
  exact?: boolean;
  /** 项目子路由项：按路由名 + params.id 匹配 */
  projectId?: string;
}

export function isMenuItemActive(item: MenuItemMatchTarget, route: MenuItemRouteLike): boolean {
  if (item.projectId) {
    return route.name === API_PROJECT_ROUTE_NAME && route.params?.id === item.projectId;
  }
  if (item.exact) {
    return route.path === item.path;
  }
  return (route.path.split('/')[1] || 'index') === item.key;
}

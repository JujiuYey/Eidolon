export interface MenuItemRouteLike {
  path: string;
  name?: unknown;
  params?: Record<string, string | string[]>;
}

/** 子路由精确匹配：路由名 + 单个 params 键 */
export interface MenuItemRouteMatch {
  name: string;
  paramKey: string;
  paramValue: string;
}

export interface MenuItemMatchTarget {
  key: string;
  path: string;
  /** 按完整 path 精确匹配（固定入口，如 /api-client），避免子路由误亮 */
  exact?: boolean;
  routeMatch?: MenuItemRouteMatch;
}

export function isMenuItemActive(item: MenuItemMatchTarget, route: MenuItemRouteLike): boolean {
  if (item.routeMatch) {
    return (
      route.name === item.routeMatch.name && route.params?.[item.routeMatch.paramKey] === item.routeMatch.paramValue
    );
  }
  if (item.exact) {
    return route.path === item.path;
  }
  return (route.path.split('/')[1] || 'index') === item.key;
}

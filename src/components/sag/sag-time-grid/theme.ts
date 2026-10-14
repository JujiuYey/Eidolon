/**
 * 画布配色：把应用的 CSS 变量翻译成 canvas 能安全使用的颜色。
 *
 * 两个坑：
 * 1. canvas 读不到 CSS 变量，取值要走 getComputedStyle；
 * 2. 项目用 oklch()，而 fillStyle 对无法解析的值会静默保留上一次的颜色——
 *    所以每个色值先画 1px 再读回像素，拿到引擎真实接受的 sRGB。
 *
 * 半透明一律用 zrender 的 fillAlpha/opacity，不在这里拼 rgba，
 * 这样底色跟着主题走，而不是钉死一个蓝色。
 */

export interface GridPalette {
  background: string;
  surface: string;
  border: string;
  borderStrong: string;
  hairline: string;
  text: string;
  textMuted: string;
  primary: string;
  destructive: string;
  muted: string;
  accent: string;
}

let probeCtx: CanvasRenderingContext2D | null = null;

/** 画一个像素再读回，把任意 CSS 颜色（oklch/color()/hsl…）归一成 rgb() */
function toCanvasColor(css: string, fallback: string): string {
  probeCtx ??= document.createElement('canvas').getContext('2d', { willReadFrequently: true });
  const ctx = probeCtx;
  if (!ctx) {
    return fallback;
  }
  const before = ctx.fillStyle;
  ctx.fillStyle = '#000000';
  ctx.fillStyle = css;
  // 解析失败时 fillStyle 保持上一个合法值，因此先比对再决定是否回退
  if (ctx.fillStyle === before && css !== '#000000') {
    return fallback;
  }
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
  ctx.fillStyle = before;
  return `rgb(${r}, ${g}, ${b})`;
}

/** 在 scope 所在文档的作用域内取一个 CSS 变量的最终颜色 */
function token(scope: Element, name: string, fallback: string): string {
  const view = scope.ownerDocument.defaultView;
  if (!view) {
    return fallback;
  }
  const raw = view.getComputedStyle(scope).getPropertyValue(name).trim();
  return raw ? toCanvasColor(raw, fallback) : fallback;
}

const FALLBACK: GridPalette = {
  background: '#ffffff',
  surface: '#ffffff',
  border: '#e4e4e7',
  borderStrong: '#d4d4d8',
  hairline: '#f4f4f5',
  text: '#18181b',
  textMuted: '#71717a',
  primary: '#2563eb',
  destructive: '#dc2626',
  muted: '#f4f4f5',
  accent: '#fafafa',
};

/** 读取网格用到的全部色值；主题切换后需重新调用 */
export function readPalette(scope: Element): GridPalette {
  return {
    background: token(scope, '--background', FALLBACK.background),
    surface: token(scope, '--card', FALLBACK.surface),
    border: token(scope, '--border', FALLBACK.border),
    borderStrong: token(scope, '--input', FALLBACK.borderStrong),
    hairline: token(scope, '--muted', FALLBACK.hairline),
    text: token(scope, '--foreground', FALLBACK.text),
    textMuted: token(scope, '--muted-foreground', FALLBACK.textMuted),
    primary: token(scope, '--primary', FALLBACK.primary),
    destructive: token(scope, '--destructive', FALLBACK.destructive),
    muted: token(scope, '--muted', FALLBACK.muted),
    accent: token(scope, '--accent', FALLBACK.accent),
  };
}

/** 主题色或明暗切换时触发重绘；返回停止监听 */
export function watchTheme(scope: Element, onChange: () => void): () => void {
  const root = scope.ownerDocument.documentElement;
  const observer = new MutationObserver(onChange);
  observer.observe(root, { attributes: true, attributeFilter: ['class', 'data-theme-color'] });
  return () => observer.disconnect();
}

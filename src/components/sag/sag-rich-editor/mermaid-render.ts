import type { Mermaid } from 'mermaid';

export interface MermaidRenderSuccess {
  ok: true;
  svg: string;
}

export interface MermaidRenderFailure {
  ok: false;
  message: string;
}

export type MermaidRenderResult = MermaidRenderSuccess | MermaidRenderFailure;

// mermaid 体积大,首个图表出现时才动态加载
let mermaidPromise: Promise<Mermaid> | null = null;
let renderSeq = 0;

async function getMermaid(isDark: boolean): Promise<Mermaid> {
  mermaidPromise ??= Promise.all([
    import('mermaid').then(mod => mod.default),
    // ELK 布局引擎(mermaid v11 起拆为外部包):flowchart 默认走 ELK,
    // 大图连线交叉明显减少;入口极小,真正的 elkjs 由其内部 loader 懒加载
    import('@mermaid-js/layout-elk').then(mod => mod.default),
  ]).then(([mermaid, elkLoaders]) => {
    mermaid.registerLayoutLoaders(elkLoaders);
    return mermaid;
  });
  const mermaid = await mermaidPromise;
  // 每次显式传全量配置:initialize 是合并语义,避免主题切换时残留旧值
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    theme: isDark ? 'dark' : 'default',
    flowchart: { defaultRenderer: 'elk' },
  });
  return mermaid;
}

function extractErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  const firstLine = message
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)[0] ?? '图表语法错误';
  return firstLine.slice(0, 160);
}

export async function renderMermaidDiagram(code: string, isDark: boolean): Promise<MermaidRenderResult> {
  try {
    const mermaid = await getMermaid(isDark);
    const { svg } = await mermaid.render(`sag-mermaid-${++renderSeq}`, code);
    return { ok: true, svg };
  } catch (error) {
    return { ok: false, message: extractErrorMessage(error) };
  }
}

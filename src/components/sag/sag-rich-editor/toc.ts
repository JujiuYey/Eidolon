/**
 * 富文本编辑器的目录抽取:从文档树收集 h1–h3 及其位置。
 * 只依赖 ProseMirror 节点的最小结构,方便单测构造假节点。
 */

export interface RichEditorTocItem {
  level: 1 | 2 | 3;
  /** 标题纯文本,空标题为空串 */
  text: string;
  /** 标题节点在文档中的位置,用于点击跳转定位 */
  pos: number;
}

/** 结构上兼容 ProseMirror 节点的最小接口 */
export interface TocSourceNode {
  type: { name: string };
  attrs: { level?: number };
  textContent: string;
}

export function collectTocItems(doc: {
  descendants(callback: (node: TocSourceNode, pos: number) => void): void;
}): RichEditorTocItem[] {
  const items: RichEditorTocItem[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== 'heading') {
      return;
    }
    const level = node.attrs.level;
    if (level !== 1 && level !== 2 && level !== 3) {
      return;
    }
    items.push({ level, text: node.textContent.trim(), pos });
  });
  return items;
}

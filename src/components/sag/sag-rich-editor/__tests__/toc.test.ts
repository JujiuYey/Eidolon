import { describe, expect, it } from 'vitest';

import { collectTocItems, type TocSourceNode } from '../toc';

interface FakeBlock {
  name: string;
  level?: number;
  text?: string;
}

/** 模拟 ProseMirror 文档:块节点位置按 开闭标签(2) + 文本长度 累计,和真实 nodeSize 规则一致 */
function fakeDoc(blocks: FakeBlock[]) {
  const entries: Array<{ node: TocSourceNode; pos: number }> = [];
  let pos = 0;
  for (const block of blocks) {
    entries.push({
      node: { type: { name: block.name }, attrs: { level: block.level }, textContent: block.text ?? '' },
      pos,
    });
    pos += 2 + (block.text?.length ?? 0);
  }
  return {
    descendants(callback: (node: TocSourceNode, pos: number) => void) {
      for (const { node, pos } of entries) {
        callback(node, pos);
      }
    },
  };
}

describe('collectTocItems', () => {
  it('收集 h1–h3 标题及其位置', () => {
    const doc = fakeDoc([
      { name: 'heading', level: 1, text: '结论速览' },
      { name: 'paragraph', text: '正文一段' },
      { name: 'heading', level: 2, text: '核对基线' },
      { name: 'heading', level: 3, text: '接口坑' },
    ]);

    expect(collectTocItems(doc)).toEqual([
      { level: 1, text: '结论速览', pos: 0 },
      { level: 2, text: '核对基线', pos: 12 },
      { level: 3, text: '接口坑', pos: 18 },
    ]);
  });

  it('跳过段落和 h4–h6', () => {
    const doc = fakeDoc([
      { name: 'paragraph', text: 'p' },
      { name: 'heading', level: 4, text: 'too deep' },
      { name: 'heading', level: 2, text: '  保留  ' },
    ]);

    expect(collectTocItems(doc)).toEqual([{ level: 2, text: '保留', pos: 13 }]);
  });

  it('空标题保留为空串,空文档返回空数组', () => {
    expect(collectTocItems(fakeDoc([{ name: 'heading', level: 1, text: '' }]))).toEqual([
      { level: 1, text: '', pos: 0 },
    ]);
    expect(collectTocItems(fakeDoc([]))).toEqual([]);
  });
});

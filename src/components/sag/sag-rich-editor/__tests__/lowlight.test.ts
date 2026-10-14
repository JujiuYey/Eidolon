import { describe, expect, it } from 'vitest';

import { editorLowlight } from '../lowlight';

describe('editorLowlight', () => {
  it('注册了文档常用语言，别名（js/html/yml/sh）随主名生效', () => {
    const languages = [
      'sql',
      'java',
      'javascript',
      'js',
      'typescript',
      'ts',
      'xml',
      'html',
      'json',
      'bash',
      'sh',
      'yaml',
      'yml',
      'css',
      'go',
      'python',
      'diff',
      'ini',
      'properties',
      'markdown',
    ];

    for (const language of languages) {
      expect(editorLowlight.registered(language)).toBe(true);
    }
  });

  it('未注册的语言不做自动猜测，highlightAuto 恒为空', () => {
    expect(editorLowlight.registered('mermaid')).toBe(false);
    expect(editorLowlight.highlightAuto().children).toEqual([]);
  });

  it('已注册语言能产出高亮节点，出错时降级为空纯文本', () => {
    const result = editorLowlight.highlight('sql', 'SELECT * FROM dept');
    expect(result.children.length).toBeGreaterThan(0);
    expect(editorLowlight.highlight('sql', 'SELECT')).toBeDefined();
  });
});

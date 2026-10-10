import { describe, expect, it } from 'vitest';

import { docsRootBasename, docsRootLabels } from '../roots';

describe('docsRootBasename', () => {
  it('取路径最后一段并兼容两种分隔符与结尾斜杠', () => {
    expect(docsRootBasename('/Users/j/Management/docs')).toBe('docs');
    expect(docsRootBasename('/Users/j/Management/docs/')).toBe('docs');
    expect(docsRootBasename('C:\\data\\notes')).toBe('notes');
    expect(docsRootBasename('docs')).toBe('docs');
  });
});

describe('docsRootLabels', () => {
  it('无重名时直接用文件夹名', () => {
    const labels = docsRootLabels([
      { id: 'a', path: '/Users/j/Management/docs' },
      { id: 'b', path: '/Users/j/Notes' },
    ]);
    expect(labels.get('a')).toBe('docs');
    expect(labels.get('b')).toBe('Notes');
  });

  it('文件夹重名时拼上一级目录区分', () => {
    const labels = docsRootLabels([
      { id: 'a', path: '/Users/j/Management/docs' },
      { id: 'b', path: '/Users/j/Work/docs' },
    ]);
    expect(labels.get('a')).toBe('Management/docs');
    expect(labels.get('b')).toBe('Work/docs');
  });

  it('拼父目录仍重名时退回完整路径', () => {
    const labels = docsRootLabels([
      { id: 'a', path: '/j/vaults/docs' },
      { id: 'b', path: '/k/vaults/docs' },
    ]);
    expect(labels.get('a')).toBe('/j/vaults/docs');
    expect(labels.get('b')).toBe('/k/vaults/docs');
  });

  it('空列表返回空映射', () => {
    expect(docsRootLabels([]).size).toBe(0);
  });
});

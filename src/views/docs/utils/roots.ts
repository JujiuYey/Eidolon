import type { DocsRoot } from '@/types/docs';

/** 取路径最后一段作为库名（兼容 / 和 \ 分隔符，忽略结尾斜杠） */
export function docsRootBasename(path: string): string {
  const segments = path.replace(/[\\/]+$/, '').split(/[\\/]/);
  return segments[segments.length - 1] ?? path;
}

function parentDirName(path: string): string {
  const segments = path.replace(/[\\/]+$/, '').split(/[\\/]/);
  return segments.length >= 2 ? (segments[segments.length - 2] ?? '') : '';
}

/**
 * 文档库显示名：默认取文件夹名；文件夹重名时拼上一级目录区分；
 * 拼了父目录仍重名的极端情况退回完整路径
 */
export function docsRootLabels(roots: DocsRoot[]): Map<string, string> {
  const baseCounts = new Map<string, number>();
  for (const root of roots) {
    const base = docsRootBasename(root.path);
    baseCounts.set(base, (baseCounts.get(base) ?? 0) + 1);
  }

  const labels = new Map<string, string>();
  const labelCounts = new Map<string, number>();
  for (const root of roots) {
    const base = docsRootBasename(root.path);
    let label = base;
    if ((baseCounts.get(base) ?? 0) > 1) {
      const parent = parentDirName(root.path);
      label = parent ? `${parent}/${base}` : base;
    }
    labels.set(root.id, label);
    labelCounts.set(label, (labelCounts.get(label) ?? 0) + 1);
  }

  for (const [id, label] of labels) {
    if ((labelCounts.get(label) ?? 0) > 1) {
      const root = roots.find(item => item.id === id);
      if (root) {
        labels.set(id, root.path);
      }
    }
  }
  return labels;
}

import { describe, expect, it } from 'vitest';

import { donutSegments } from '../chart';

describe('donutSegments', () => {
  it('sorts segments descending and starts the largest at 12 oclock', () => {
    const { segments, total } = donutSegments([
      { label: 'GET', value: 10 },
      { label: 'POST', value: 30 },
      { label: 'DELETE', value: 10 },
    ]);

    expect(total).toBe(50);
    expect(segments.map(segment => segment.label)).toEqual(['POST', 'GET', 'DELETE']);

    // 最大段：占 60%，从 12 点方向开始（offset 25 把 3 点钟起点搬到 12 点）
    expect(segments[0]).toMatchObject({
      fraction: 0.6,
      percentLabel: '60%',
      dashArray: '60 40',
      dashOffset: '25',
    });
    // 第二段从 60% 处开始：offset = (25 - 60) mod 100 = 65
    expect(segments[1]).toMatchObject({
      fraction: 0.2,
      dashArray: '20 80',
      dashOffset: '65',
    });
    // 第三段从 80% 处开始：offset = (25 - 80) mod 100 = 45
    expect(segments[2]).toMatchObject({
      fraction: 0.2,
      dashOffset: '45',
    });
  });

  it('renders a single segment as a full circle', () => {
    const { segments } = donutSegments([{ label: '成功', value: 7 }]);
    expect(segments).toHaveLength(1);
    expect(segments[0]).toMatchObject({
      fraction: 1,
      percentLabel: '100%',
      dashArray: '100 0',
      dashOffset: '25',
    });
  });

  it('skips zero-value segments and returns empty math for empty input', () => {
    const { segments } = donutSegments([
      { label: 'A', value: 0 },
      { label: 'B', value: 5 },
    ]);
    expect(segments).toHaveLength(1);
    expect(segments[0]?.label).toBe('B');

    expect(donutSegments([])).toEqual({ segments: [], total: 0 });
    expect(donutSegments([{ label: 'A', value: 0 }])).toEqual({ segments: [], total: 0 });
  });
});

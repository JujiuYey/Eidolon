/**
 * 图表几何计算（纯函数，可单测）。
 *
 * 环形图用 SVG stroke-dasharray 百分比技巧：r = 100/(2π) 时周长恰为 100，
 * 每段 dashArray 直接用百分数表示；dashOffset 把段落起点从 3 点钟方向
 * 搬到 12 点钟方向（最大扇区对齐 12 点，见 ui-ux-pro-max chart 指引）。
 */

export interface DonutSegmentInput {
  label: string;
  value: number;
}

export interface DonutSegmentMath {
  label: string;
  value: number;
  /** 占比 0~1 */
  fraction: number;
  /** '37%' 形式的百分比文案 */
  percentLabel: string;
  /** SVG stroke-dasharray（周长按 100 计） */
  dashArray: string;
  /** SVG stroke-dashoffset（已归一化到 0~100） */
  dashOffset: string;
}

export interface DonutMath {
  segments: DonutSegmentMath[];
  total: number;
}

/**
 * 计算环形图各段的 dash 参数。零值段跳过；按数值降序（最大段从 12 点开始）；
 * 总量为 0 时返回空段列表。
 */
export function donutSegments(items: DonutSegmentInput[]): DonutMath {
  const total = items.reduce((sum, item) => sum + item.value, 0);
  if (total <= 0) {
    return { segments: [], total: 0 };
  }

  const sorted = [...items].sort((a, b) => b.value - a.value);
  let consumed = 0;
  const segments = sorted
    .filter(item => item.value > 0)
    .map(item => {
      const fraction = item.value / total;
      const startPercent = consumed * 100;
      consumed += fraction;

      const dashArray = `${fraction * 100} ${100 - fraction * 100}`;
      const dashOffset = (((25 - startPercent) % 100) + 100) % 100;

      return {
        label: item.label,
        value: item.value,
        fraction,
        percentLabel: `${Math.round(fraction * 100)}%`,
        dashArray,
        dashOffset: `${dashOffset}`,
      };
    });

  return { segments, total };
}

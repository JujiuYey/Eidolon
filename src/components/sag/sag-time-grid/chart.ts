/**
 * 场景绘制：把「日列 + 时间块」画成一帧 zrender 图元。
 *
 * 与 demo 的 modules/ 同构——绘制只负责把状态变成图元，不持有交互状态；
 * 交互由 index.vue 的事件层处理。每帧整体重建：日程块是几十级别，
 * 重建成本远低于维护增量更新。
 *
 * 配色全部来自 theme.ts 的应用主题变量，画布本身不含任何硬编码色值。
 */

import * as zrender from 'zrender';

import {
  computeLanes,
  durationToHeight,
  formatMinutes,
  GUTTER_WIDTH,
  type TimeBlock,
} from '@/views/tasks/utils/schedule';

import { blockRect, dayLeft, HEADER_HEIGHT, minutesToY, type BlockRect } from './geometry';
import type { GridPalette } from './theme';

const WEEKDAYS = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];

const FONT_LABEL = '10px ui-sans-serif, system-ui, sans-serif';
const FONT_TIME = '11px ui-sans-serif, system-ui, sans-serif';
const FONT_TITLE = '600 11px ui-sans-serif, system-ui, sans-serif';
const FONT_TITLE_DONE = '11px ui-sans-serif, system-ui, sans-serif';
const FONT_WEEKDAY = '500 11px ui-sans-serif, system-ui, sans-serif';

export interface ChartInput {
  days: string[];
  blocks: TimeBlock[];
  todayKey: string;
  nowMs: number;
  palette: GridPalette;
  /** 正在拖动的块：用预览位置覆盖真实位置 */
  preview: { id: string; dayIndex: number; start: number; end: number } | null;
  /** 悬停的块：强调描边并显出手柄 */
  hoverId: string | null;
  /** 空白处框选/新建预览 */
  selection: { dayIndex: number; start: number; end: number } | null;
  /** 左栏拖入预览 */
  externalPreview: { dayIndex: number; start: number; duration: number } | null;
}

export interface BuiltScene {
  root: zrender.Group;
  /** blockId → 矩形，供事件层做命中测试 */
  rects: Map<string, BlockRect>;
}

function rect(
  x: number,
  y: number,
  width: number,
  height: number,
  fill: string,
  style: zrender.PathStyleProps = {},
  radius = 0,
): zrender.Rect {
  return new zrender.Rect({ shape: { x, y, width, height, r: radius }, style: { fill, ...style } });
}

function label(value: string, x: number, y: number, style: zrender.TextStyleProps = {}): zrender.Text {
  return new zrender.Text({ style: { x, y, text: value, font: FONT_LABEL, fill: '#000', ...style } });
}

function silent<T extends zrender.Element>(element: T): T {
  element.silent = true;
  return element;
}

/** 同一天内重叠的块并排；跨日互不影响 */
function lanesByDay(blocks: TimeBlock[]): Map<string, { lane: number; lanes: number }> {
  const grouped = new Map<string, TimeBlock[]>();
  for (const block of blocks) {
    const list = grouped.get(block.date);
    if (list) {
      list.push(block);
    } else {
      grouped.set(block.date, [block]);
    }
  }
  const result = new Map<string, { lane: number; lanes: number }>();
  for (const list of grouped.values()) {
    for (const [id, lane] of computeLanes(list)) {
      result.set(id, lane);
    }
  }
  return result;
}

function drawHeader(root: zrender.Group, input: ChartInput, dayW: number) {
  const { palette } = input;
  const header = new zrender.Group();
  header.add(silent(rect(0, 0, GUTTER_WIDTH + input.days.length * dayW, HEADER_HEIGHT, palette.background)));

  input.days.forEach((key, index) => {
    const left = dayLeft(index, dayW);
    const isToday = key === input.todayKey;
    const date = new Date(`${key}T00:00`);
    const weekday = WEEKDAYS[(date.getDay() + 6) % 7] ?? '';
    const dateLabel = `${date.getMonth() + 1}/${date.getDate()}`;
    const width = Math.max(dayW - 12, 10);

    // 今天用主色底条建立层级，而不是给整列上底色
    if (isToday) {
      header.add(silent(rect(left + 6, 7, 3, HEADER_HEIGHT - 17, palette.primary, undefined, 2)));
    }
    header.add(
      silent(
        label(weekday, left + (isToday ? 15 : 8), 11, {
          font: isToday ? FONT_WEEKDAY : FONT_LABEL,
          fill: isToday ? palette.primary : palette.textMuted,
          width,
          overflow: 'truncate',
        }),
      ),
    );
    header.add(
      silent(
        label(dateLabel, left + (isToday ? 15 : 8), 24, {
          font: FONT_TIME,
          fill: isToday ? palette.text : palette.textMuted,
          width,
          overflow: 'truncate',
        }),
      ),
    );
    if (index > 0) {
      header.add(silent(rect(left, 8, 1, HEADER_HEIGHT - 20, palette.border)));
    }
  });

  header.add(silent(rect(0, HEADER_HEIGHT - 1, GUTTER_WIDTH + input.days.length * dayW, 1, palette.border)));
  root.add(header);
}

function drawTimeGrid(root: zrender.Group, input: ChartInput, dayW: number, chartH: number) {
  const { palette } = input;
  const grid = new zrender.Group();
  const todayIndex = input.days.indexOf(input.todayKey);

  // 今日列只留极淡的一层底，靠表头的主色条指认，避免整列抢戏
  if (todayIndex >= 0) {
    grid.add(
      silent(
        rect(dayLeft(todayIndex, dayW), HEADER_HEIGHT, dayW, chartH - HEADER_HEIGHT, palette.primary, {
          opacity: 0.04,
        }),
      ),
    );
  }

  input.days.forEach((_, index) => {
    if (index > 0) {
      grid.add(silent(rect(dayLeft(index, dayW), HEADER_HEIGHT, 1, chartH - HEADER_HEIGHT, palette.border)));
    }
  });

  for (let hour = 0; hour <= 24; hour += 1) {
    const y = minutesToY(hour * 60);
    const major = hour % 2 === 0;
    grid.add(
      silent(
        rect(GUTTER_WIDTH, y, input.days.length * dayW, 1, major ? palette.border : palette.hairline, {
          opacity: major ? 1 : 0.6,
        }),
      ),
    );
    if (hour < 24) {
      grid.add(
        silent(
          label(`${String(hour).padStart(2, '0')}:00`, 4, y - 6, {
            font: FONT_LABEL,
            fill: palette.textMuted,
            opacity: major ? 0.9 : 0.45,
          }),
        ),
      );
    }
  }

  root.add(grid);
}

interface BlockSkin {
  fill: string;
  opacity: number;
  stroke: string;
  title: string;
  time: string;
}

function blockSkin(block: TimeBlock, palette: GridPalette): BlockSkin {
  if (block.status === 'done') {
    return {
      fill: palette.muted,
      opacity: 0.7,
      stroke: 'transparent',
      title: palette.textMuted,
      time: palette.textMuted,
    };
  }
  if (block.status === 'doing') {
    return {
      fill: palette.primary,
      opacity: 0.12,
      stroke: palette.primary,
      title: palette.text,
      time: palette.textMuted,
    };
  }
  return {
    fill: palette.surface,
    opacity: 1,
    stroke: palette.borderStrong,
    title: palette.text,
    time: palette.textMuted,
  };
}

function drawBlock(block: TimeBlock, box: BlockRect, input: ChartInput): zrender.Group {
  const { palette } = input;
  const group = new zrender.Group();
  const skin = blockSkin(block, palette);
  const dragging = input.preview?.id === block.id;
  const hovered = input.hoverId === block.id;
  const inset = 1;

  // 结构用发丝描边表达；只有真正「浮起来」的拖动态才给阴影（R10：两者不叠加）
  group.add(
    rect(
      box.x + inset,
      box.y + inset,
      box.width - inset * 2,
      box.height - inset * 2,
      skin.fill,
      {
        opacity: skin.opacity,
        stroke: block.overdue ? palette.destructive : dragging || hovered ? palette.primary : skin.stroke,
        lineWidth: block.overdue || dragging || hovered ? 1.5 : 1,
        shadowBlur: dragging ? 12 : 0,
        shadowColor: 'rgba(0, 0, 0, 0.18)',
        shadowOffsetY: dragging ? 3 : 0,
      },
      5,
    ),
  );

  const compact = box.height < durationToHeight(45);
  const padX = box.x + 7;
  const innerWidth = Math.max(box.width - 14, 10);
  group.add(
    label(block.title, padX, box.y + 5, {
      font: block.status === 'done' ? FONT_TITLE_DONE : FONT_TITLE,
      fill: skin.title,
      opacity: block.status === 'done' ? 0.7 : 1,
      width: innerWidth,
      overflow: 'truncate',
    }),
  );
  if (!compact) {
    group.add(
      label(`${formatMinutes(block.start)}–${formatMinutes(block.end)}`, padX, box.y + 19, {
        font: FONT_LABEL,
        fill: skin.time,
        width: innerWidth,
        overflow: 'truncate',
      }),
    );
  }

  // 逾期用左侧色条标记，和标题并列但不喧宾夺主
  if (block.overdue) {
    group.add(
      silent(
        rect(
          box.x + inset,
          box.y + inset + 2,
          3,
          Math.max(box.height - inset * 2 - 4, 4),
          palette.destructive,
          undefined,
          1.5,
        ),
      ),
    );
  }

  // 手柄：默认隐形，悬停/拖动时才显形，避免整屏灰点
  if (hovered || dragging) {
    const handle = 'rgba(255, 255, 255, 0.85)';
    group.add(rect(box.x + box.width / 2 - 9, box.y + 3, 18, 2, handle, { opacity: 0.9 }, 1));
    group.add(rect(box.x + box.width / 2 - 9, box.y + box.height - 5, 18, 2, handle, { opacity: 0.9 }, 1));
  }

  return group;
}

function drawNowLine(root: zrender.Group, input: ChartInput, dayW: number) {
  const dayIndex = input.days.indexOf(input.todayKey);
  if (!input.nowMs || dayIndex < 0) {
    return;
  }
  const now = new Date(input.nowMs);
  const y = minutesToY(now.getHours() * 60 + now.getMinutes());
  const left = dayLeft(dayIndex, dayW);
  root.add(silent(rect(left, y - 1, dayW, 2, input.palette.primary)));
  root.add(silent(new zrender.Circle({ shape: { cx: left, cy: y, r: 3.5 }, style: { fill: input.palette.primary } })));
}

/** 构建一帧场景并挂到 zrender 上；返回块矩形供事件层命中 */
export function buildScene(zr: zrender.ZRenderType, input: ChartInput): BuiltScene {
  const root = new zrender.Group();
  const dayCount = input.days.length;
  const chartW = zr.getWidth() ?? 0;
  const chartH = zr.getHeight() ?? 0;
  const dayW = (chartW - GUTTER_WIDTH) / dayCount;

  root.add(silent(rect(0, 0, chartW, chartH, input.palette.background)));
  drawHeader(root, input, dayW);
  drawTimeGrid(root, input, dayW, chartH);

  const blocks = input.blocks.map(block => {
    const preview = input.preview;
    if (!preview || preview.id !== block.id) {
      return block;
    }
    const date = input.days[preview.dayIndex];
    return date ? { ...block, date, start: preview.start, end: preview.end } : block;
  });

  const lanes = lanesByDay(blocks);
  const rects = new Map<string, BlockRect>();
  const blocksGroup = new zrender.Group();
  for (const block of blocks) {
    const dayIndex = input.days.indexOf(block.date);
    if (dayIndex < 0) {
      continue;
    }
    const lane = lanes.get(block.id) ?? { lane: 0, lanes: 1 };
    const box = blockRect(dayIndex, lane.lane, lane.lanes, dayW, block.start, block.end);
    rects.set(block.id, box);
    blocksGroup.add(drawBlock(block, box, input));
  }
  root.add(blocksGroup);

  if (input.selection) {
    const box = blockRect(input.selection.dayIndex, 0, 1, dayW, input.selection.start, input.selection.end);
    root.add(
      silent(
        rect(
          box.x,
          box.y,
          box.width,
          box.height,
          input.palette.primary,
          {
            opacity: 0.1,
            stroke: input.palette.primary,
            lineWidth: 1,
            lineDash: [4, 3],
          },
          5,
        ),
      ),
    );
  }

  if (input.externalPreview) {
    const box = blockRect(
      input.externalPreview.dayIndex,
      0,
      1,
      dayW,
      input.externalPreview.start,
      input.externalPreview.start + input.externalPreview.duration,
    );
    root.add(
      silent(
        rect(
          box.x,
          box.y,
          box.width,
          box.height,
          input.palette.primary,
          {
            opacity: 0.12,
            stroke: input.palette.primary,
            lineWidth: 1.5,
          },
          5,
        ),
      ),
    );
    root.add(
      silent(
        label(`${formatMinutes(input.externalPreview.start)} 起`, box.x + 7, box.y + 5, {
          font: FONT_TITLE,
          fill: input.palette.primary,
        }),
      ),
    );
  }

  if (input.blocks.length === 0) {
    const hint = '从左侧把任务拖到这里，或在空白处拖一段新建';
    root.add(
      silent(
        label(hint, chartW / 2 - hint.length * 5, minutesToY(10 * 60), {
          font: FONT_TIME,
          fill: input.palette.textMuted,
          opacity: 0.75,
        }),
      ),
    );
  }

  drawNowLine(root, input, dayW);
  zr.add(root);
  return { root, rects };
}

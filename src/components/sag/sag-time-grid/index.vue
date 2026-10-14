<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import * as zrender from 'zrender';

import { GUTTER_WIDTH, MINUTES_PER_DAY, SNAP_MINUTES, type TimeBlock } from '@/views/tasks/utils/schedule';

import { buildScene } from './chart';
import {
  CHART_HEIGHT,
  dayWidth,
  HEADER_HEIGHT,
  MIN_DAY_WIDTH,
  resizeEnd,
  resizeStart,
  snapDrop,
  xToDayIndex,
  type BlockRect,
} from './geometry';
import { readPalette, watchTheme, type GridPalette } from './theme';

/**
 * 日程时间网格（canvas + zrender 渲染）。
 *
 * 交互只有一套指针事件流：mousedown → mousemove → mouseup，
 * 块内拖动、上下手柄缩放、空白框选新建、点按打开全部走它；
 * 命中测试用 buildScene 返回的矩形表，不依赖 zrender 的元素事件，
 * 因此拖到画布外也能继续收到 window 上的 move/up。
 *
 * 左栏拖入是唯一例外——它从 DOM 发起，canvas 只能借 HTML5 DnD 接收。
 */

const props = withDefaults(
  defineProps<{
    days: string[];
    blocks: TimeBlock[];
    todayKey?: string;
    nowMs?: number;
    externalDragActive?: boolean;
    externalDuration?: number;
  }>(),
  { todayKey: '', nowMs: 0, externalDragActive: false, externalDuration: 30 },
);

const emit = defineEmits<{
  (e: 'move-block', id: string, date: string, start: number, end: number): void;
  (e: 'resize-block', id: string, start: number, end: number): void;
  (e: 'create-block', date: string, start: number, end: number, x: number, y: number): void;
  (e: 'click-block', id: string): void;
  (e: 'toggle-block', id: string): void;
  (e: 'block-drag-outside', id: string | null): void;
  (e: 'block-drop-outside', id: string, x: number, y: number): void;
  (e: 'external-drop', date: string, start: number): void;
}>();

const scrollEl = ref<HTMLDivElement | null>(null);
const canvasEl = ref<HTMLCanvasElement | null>(null);

const zr = shallowRef<zrender.ZRenderType | null>(null);
/** 画布配色跟随应用主题；主题色或明暗切换时重新读取 */
const palette = ref<GridPalette | null>(null);
const chartWidth = ref(GUTTER_WIDTH + 7 * MIN_DAY_WIDTH);
/** 最近一帧的块矩形，命中测试的唯一依据 */
const blockRects = shallowRef(new Map<string, BlockRect>());

type Mode = 'move' | 'resize-top' | 'resize-bottom' | 'select';

interface Interaction {
  mode: Mode;
  /** move/resize 时的块 id；select 为空串 */
  id: string;
  /** 按下瞬间的分钟数，换算落点时要减去它得到抓取偏移 */
  grabMinutes: number;
  originDayIndex: number;
  originStart: number;
  originEnd: number;
  moved: boolean;
}

const interaction = ref<Interaction | null>(null);
const preview = ref<{ id: string; dayIndex: number; start: number; end: number } | null>(null);
const selection = ref<{ dayIndex: number; start: number; end: number } | null>(null);
const externalPreview = ref<{ dayIndex: number; start: number } | null>(null);
/** 悬停块：仅在 id 变化时触发重绘，避免 mousemove 里反复清屏重画 */
const hoverId = ref<string | null>(null);

// ===== 坐标 =====

const dayCount = () => Math.max(props.days.length, 1);
const dayW = () => dayWidth(chartWidth.value, dayCount());

function clientToCanvas(clientX: number, clientY: number): { x: number; y: number } {
  const rect = canvasEl.value?.getBoundingClientRect();
  if (!rect) {
    return { x: 0, y: 0 };
  }
  return { x: clientX - rect.left, y: clientY - rect.top };
}

function canvasToMinutes(y: number): number {
  const ratio = (y - HEADER_HEIGHT) / (CHART_HEIGHT - HEADER_HEIGHT);
  return Math.max(0, Math.min(MINUTES_PER_DAY, ratio * MINUTES_PER_DAY));
}

function snap(value: number): number {
  return Math.round(value / SNAP_MINUTES) * SNAP_MINUTES;
}

function insideCanvas(x: number, y: number): boolean {
  return x >= 0 && x <= chartWidth.value && y >= 0 && y <= CHART_HEIGHT;
}

/** 命中最上层（面积最小）的块 */
function hitBlock(x: number, y: number): string | null {
  let best: { id: string; area: number } | null = null;
  for (const [id, box] of blockRects.value) {
    if (x < box.x || x >= box.x + box.width || y < box.y || y >= box.y + box.height) {
      continue;
    }
    const area = box.width * box.height;
    if (!best || area < best.area) {
      best = { id, area };
    }
  }
  return best?.id ?? null;
}

// ===== 渲染 =====

function render() {
  const instance = zr.value;
  const colors = palette.value;
  if (!instance || !colors) {
    return;
  }
  instance.clear();
  const scene = buildScene(instance, {
    palette: colors,
    days: props.days,
    blocks: props.blocks,
    todayKey: props.todayKey,
    nowMs: props.nowMs,
    preview: preview.value,
    hoverId: hoverId.value,
    selection: selection.value,
    externalPreview: externalPreview.value ? { ...externalPreview.value, duration: props.externalDuration } : null,
  });
  blockRects.value = scene.rects;
}

function resizeCanvas() {
  const scroller = scrollEl.value;
  if (!scroller) {
    return;
  }
  chartWidth.value = Math.max(GUTTER_WIDTH + dayCount() * MIN_DAY_WIDTH, scroller.clientWidth);
  zr.value?.resize({ width: chartWidth.value, height: CHART_HEIGHT });
  render();
}

// ===== 指针交互 =====

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  hoverId.value = null;
  const { x, y } = clientToCanvas(event.clientX, event.clientY);
  if (!insideCanvas(x, y)) {
    return;
  }
  const minutes = canvasToMinutes(y);
  const id = hitBlock(x, y);

  if (id) {
    const block = props.blocks.find(item => item.id === id);
    if (!block) {
      return;
    }
    const box = blockRects.value.get(id)!;
    const edge = y - box.y <= 5 ? 'resize-top' : box.y + box.height - y <= 5 ? 'resize-bottom' : 'move';
    interaction.value = {
      mode: edge,
      id,
      grabMinutes: minutes - block.start,
      originDayIndex: props.days.indexOf(block.date),
      originStart: block.start,
      originEnd: block.end,
      moved: false,
    };
  } else if (x >= GUTTER_WIDTH) {
    interaction.value = {
      mode: 'select',
      id: '',
      grabMinutes: 0,
      originDayIndex: xToDayIndex(x, dayW(), dayCount()),
      originStart: snap(minutes),
      originEnd: snap(minutes),
      moved: false,
    };
  }

  if (interaction.value) {
    event.preventDefault();
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
  }
}

function onPointerMove(event: PointerEvent) {
  const state = interaction.value;
  if (!state) {
    const { x, y } = clientToCanvas(event.clientX, event.clientY);
    const next = insideCanvas(x, y) ? hitBlock(x, y) : null;
    if (next !== hoverId.value) {
      hoverId.value = next;
      render();
    }
    return;
  }
  const { x, y } = clientToCanvas(event.clientX, event.clientY);
  state.moved = true;

  if (state.mode === 'select') {
    const current = snap(canvasToMinutes(y));
    if (Math.abs(current - state.originStart) >= SNAP_MINUTES) {
      selection.value = {
        dayIndex: xToDayIndex(x, dayW(), dayCount()),
        start: Math.min(state.originStart, current),
        end: Math.max(state.originStart, current),
      };
    }
    render();
    return;
  }

  const inside = insideCanvas(x, y);
  if (state.mode === 'move') {
    if (!inside) {
      emit('block-drag-outside', state.id);
      return;
    }
    // 用抓取偏移还原块顶，再吸附——避免长块被拖到边缘时整体跳变
    const offsetPx = state.grabMinutes * ((CHART_HEIGHT - HEADER_HEIGHT) / MINUTES_PER_DAY);
    const dropped = snapDrop(y - offsetPx, state.originEnd - state.originStart);
    preview.value = {
      id: state.id,
      dayIndex: xToDayIndex(x, dayW(), dayCount()),
      start: dropped.start,
      end: dropped.end,
    };
  } else if (inside) {
    const next =
      state.mode === 'resize-bottom'
        ? { start: state.originStart, end: resizeEnd(y, state.originStart) }
        : { start: resizeStart(y, state.originEnd), end: state.originEnd };
    preview.value = { id: state.id, dayIndex: state.originDayIndex, start: next.start, end: next.end };
  }

  render();
}

/** 指针移出画布时通知外层（左栏高亮），回到画布内要撤销 */
const lastOutside = ref(false);

function syncOutside(inside: boolean) {
  const state = interaction.value;
  if (!state || state.mode !== 'move' || !state.id) {
    return;
  }
  if (inside === lastOutside.value) {
    return;
  }
  lastOutside.value = !inside;
  emit('block-drag-outside', inside ? null : state.id);
}

function onPointerUp(event: PointerEvent) {
  const state = interaction.value;
  interaction.value = null;
  lastOutside.value = false;
  if (!state) {
    return;
  }
  const { x, y } = clientToCanvas(event.clientX, event.clientY);
  hoverId.value = insideCanvas(x, y) ? hitBlock(x, y) : null;
  const dropped = preview.value;
  const picked = selection.value;
  preview.value = null;
  selection.value = null;
  emit('block-drag-outside', null);
  render();

  if (state.mode === 'select') {
    const date = props.days[picked?.dayIndex ?? state.originDayIndex] ?? '';
    if (!state.moved) {
      emit(
        'create-block',
        date,
        state.originStart,
        Math.min(state.originStart + 30, MINUTES_PER_DAY),
        event.clientX,
        event.clientY,
      );
      return;
    }
    if (picked) {
      emit('create-block', date, picked.start, picked.end, event.clientX, event.clientY);
    }
    return;
  }

  if (!state.moved) {
    emit('click-block', state.id);
    return;
  }

  if (state.mode === 'move' && !insideCanvas(x, y)) {
    emit('block-drop-outside', state.id, event.clientX, event.clientY);
    return;
  }
  if (!dropped) {
    return;
  }
  if (state.mode === 'move') {
    emit('move-block', state.id, props.days[dropped.dayIndex] ?? '', dropped.start, dropped.end);
  } else {
    emit('resize-block', state.id, dropped.start, dropped.end);
  }
}

function onPointerCancel() {
  interaction.value = null;
  preview.value = null;
  selection.value = null;
  lastOutside.value = false;
  emit('block-drag-outside', null);
  render();
}

/** window 级监听：拖到画布外也要继续收到 move/up */
function onWindowMove(event: PointerEvent) {
  if (!interaction.value) {
    return;
  }
  const { x, y } = clientToCanvas(event.clientX, event.clientY);
  syncOutside(!insideCanvas(x, y) && interaction.value.mode === 'move');
}

function onWindowUp(event: PointerEvent) {
  if (interaction.value) {
    onPointerUp(event);
  }
}

// ===== 左栏 HTML5 拖入 =====

function onDragOver(event: DragEvent) {
  if (!props.externalDragActive) {
    return;
  }
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
  const { x, y } = clientToCanvas(event.clientX, event.clientY);
  externalPreview.value = { dayIndex: xToDayIndex(x, dayW(), dayCount()), start: snap(canvasToMinutes(y)) };
  render();
}

function onDragLeave() {
  if (!externalPreview.value) {
    return;
  }
  externalPreview.value = null;
  render();
}

function onDrop(event: DragEvent) {
  event.preventDefault();
  const spot = externalPreview.value;
  externalPreview.value = null;
  render();
  if (spot) {
    emit('external-drop', props.days[spot.dayIndex] ?? '', spot.start);
  }
}

// ===== 生命周期 =====

onMounted(() => {
  const canvas = canvasEl.value;
  if (!canvas) {
    return;
  }
  const instance = zrender.init(canvas, { devicePixelRatio: window.devicePixelRatio });
  zr.value = instance;
  palette.value = readPalette(canvas);
  stopThemeWatch = watchTheme(canvas, () => {
    palette.value = readPalette(canvas);
    render();
  });
  instance.resize({ width: chartWidth.value, height: CHART_HEIGHT });
  resizeCanvas();
  // 初始滚到工作时段，让上午落在视口上部
  if (scrollEl.value) {
    scrollEl.value.scrollTop = HEADER_HEIGHT + (7.5 * (CHART_HEIGHT - HEADER_HEIGHT)) / (MINUTES_PER_DAY / 60);
  }
  window.addEventListener('pointermove', onWindowMove);
  window.addEventListener('pointerup', onWindowUp);
});

let stopThemeWatch: (() => void) | null = null;

onBeforeUnmount(() => {
  stopThemeWatch?.();
  window.removeEventListener('pointermove', onWindowMove);
  window.removeEventListener('pointerup', onWindowUp);
  zr.value?.dispose();
  zr.value = null;
});

watch(
  () => [props.days, props.blocks, props.todayKey, props.nowMs] as const,
  () => render(),
  { deep: true },
);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-background">
    <div ref="scrollEl" class="min-h-0 flex-1 overflow-auto">
      <div class="relative" :style="{ width: `${chartWidth}px`, height: `${CHART_HEIGHT}px` }">
        <canvas
          ref="canvasEl"
          class="absolute left-0 top-0 touch-none"
          :style="{ width: `${chartWidth}px`, height: `${CHART_HEIGHT}px` }"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerCancel"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
        />
      </div>
    </div>
  </div>
</template>

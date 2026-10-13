<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

import {
  GUTTER_WIDTH,
  GRID_HEIGHT,
  MINUTES_PER_DAY,
  PX_PER_HOUR,
  SNAP_MINUTES,
  computeLanes,
  durationToHeight,
  formatMinutes,
  minutesToOffset,
  snapMinutes,
  type TimeBlock,
} from '@/views/tasks/utils/schedule';

/**
 * 日程时间网格（周/日视图）。
 *
 * 自己拥有全部指针交互：块拖动、上下边缘缩放、空白框选、键盘 ±15 分钟微调；
 * 左栏清单的拖入走 HTML5 DnD（dragover 预览 + drop 落点）。时间计算全部委托 schedule.ts 纯函数。
 */

const props = withDefaults(
  defineProps<{
    /** 要渲染的日期键（YYYY-MM-DD），周视图 7 个、日视图 1 个 */
    days: string[];
    /** 本周所有时间块（跨日的由本组件按日分拣） */
    blocks: TimeBlock[];
    /** 今天（YYYY-MM-DD），无则不渲染今日列与 now 线 */
    todayKey?: string;
    /** 当前时间戳（毫秒），驱动 now 线 */
    nowMs?: number;
    /** 左栏任务拖入进行中（HTML5 DnD） */
    externalDragActive?: boolean;
    /** 拖入预览的块时长（分钟） */
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

const scrollEl = ref<HTMLElement | null>(null);
const contentEl = ref<HTMLElement | null>(null);

// ===== 布局几何 =====

const dayCount = computed(() => props.days.length);
const minWidth = computed(() => `${GUTTER_WIDTH + dayCount.value * 92}px`);

const WEEKDAY_LABELS = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];

interface DayHeader {
  key: string;
  weekday: string;
  dayOfMonth: string;
  isToday: boolean;
}

const dayHeaders = computed<DayHeader[]>(() =>
  props.days.map(key => {
    const date = new Date(`${key}T00:00`);
    return {
      key,
      weekday: WEEKDAY_LABELS[(date.getDay() + 6) % 7] ?? '',
      dayOfMonth: `${date.getMonth() + 1}/${date.getDate()}`,
      isToday: key === props.todayKey,
    };
  }),
);

const hourLines = Array.from({ length: 23 }, (_, index) => (index + 1) * PX_PER_HOUR);
const halfHourLines = Array.from({ length: 24 }, (_, index) => index * PX_PER_HOUR + PX_PER_HOUR / 2);

// ===== 指针拖拽状态机 =====

interface DragState {
  kind: 'move' | 'resize-top' | 'resize-bottom' | 'select';
  id: string;
  pointerId: number;
  originDayIndex: number;
  grabOffset: number;
  duration: number;
  originStart: number;
  originEnd: number;
  dayIndex: number;
  start: number;
  end: number;
  moved: boolean;
  outside: boolean;
}

const drag = ref<DragState | null>(null);
const dragPointer = ref({ x: 0, y: 0 });

function pointToMinutes(clientX: number, clientY: number): { minutes: number; dayIndex: number } | null {
  const el = contentEl.value;
  if (!el) {
    return null;
  }
  const rect = el.getBoundingClientRect();
  const minutes = ((clientY - rect.top) / PX_PER_HOUR) * 60;
  const dayWidth = (rect.width - GUTTER_WIDTH) / dayCount.value;
  const rawIndex = Math.floor((clientX - rect.left - GUTTER_WIDTH) / dayWidth);
  const dayIndex = Math.max(0, Math.min(dayCount.value - 1, rawIndex));
  return { minutes, dayIndex };
}

function isInsideContent(clientX: number, clientY: number): boolean {
  const rect = contentEl.value?.getBoundingClientRect();
  if (!rect) {
    return false;
  }
  return clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom;
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  const target = event.target as HTMLElement;
  const point = pointToMinutes(event.clientX, event.clientY);
  if (!point) {
    return;
  }

  const blockEl = target.closest<HTMLElement>('[data-block-id]');
  const snapped = snapMinutes(point.minutes);

  if (blockEl) {
    const id = blockEl.dataset.blockId!;
    const block = props.blocks.find(item => item.id === id);
    if (!block) {
      return;
    }
    const resize = target.closest<HTMLElement>('[data-resize]')?.dataset.resize;
    event.preventDefault();
    blockEl.focus({ preventScroll: true });
    contentEl.value?.setPointerCapture(event.pointerId);
    drag.value = {
      kind: resize === 'top' ? 'resize-top' : resize === 'bottom' ? 'resize-bottom' : 'move',
      id,
      pointerId: event.pointerId,
      originDayIndex: props.days.indexOf(block.date),
      grabOffset: snapMinutes(point.minutes - block.start),
      duration: block.end - block.start,
      originStart: block.start,
      originEnd: block.end,
      dayIndex: props.days.indexOf(block.date),
      start: block.start,
      end: block.end,
      moved: false,
      outside: false,
    };
    return;
  }

  // 空白处：框选（没拖动就是单击，统一走 create-block）；时间刻度条上的点击忽略
  const rect = contentEl.value?.getBoundingClientRect();
  if (rect && event.clientX - rect.left < GUTTER_WIDTH) {
    return;
  }
  event.preventDefault();
  contentEl.value?.setPointerCapture(event.pointerId);
  drag.value = {
    kind: 'select',
    id: '',
    pointerId: event.pointerId,
    originDayIndex: point.dayIndex,
    grabOffset: 0,
    duration: 0,
    originStart: snapped,
    originEnd: snapped,
    dayIndex: point.dayIndex,
    start: snapped,
    end: snapped,
    moved: false,
    outside: false,
  };
}

function onPointerMove(event: PointerEvent) {
  const state = drag.value;
  if (!state || state.pointerId !== event.pointerId) {
    return;
  }
  dragPointer.value = { x: event.clientX, y: event.clientY };
  const point = pointToMinutes(event.clientX, event.clientY);
  if (!point) {
    return;
  }

  if (!state.moved) {
    state.moved = true;
  }

  if (state.kind === 'select') {
    const current = snapMinutes(point.minutes);
    state.start = Math.min(state.originStart, current);
    state.end = Math.max(state.originStart, current);
    return;
  }

  const outside = !isInsideContent(event.clientX, event.clientY);
  if (state.kind === 'move' && outside !== state.outside) {
    state.outside = outside;
    emit('block-drag-outside', outside ? state.id : null);
  }
  if (outside) {
    return; // 移出网格期间冻结位置，等回来或松手
  }

  if (state.kind === 'move') {
    state.dayIndex = point.dayIndex;
    const duration = Math.max(SNAP_MINUTES, state.duration);
    let start = snapMinutes(point.minutes - state.grabOffset);
    start = Math.min(start, MINUTES_PER_DAY - duration);
    state.start = start;
    state.end = start + duration;
    return;
  }

  const current = snapMinutes(point.minutes);
  if (state.kind === 'resize-bottom') {
    state.end = Math.max(state.start + SNAP_MINUTES, Math.min(MINUTES_PER_DAY, current));
  } else {
    state.start = Math.min(state.end - SNAP_MINUTES, Math.max(0, current));
  }
}

function onPointerUp(event: PointerEvent) {
  const state = drag.value;
  if (!state || state.pointerId !== event.pointerId) {
    return;
  }
  drag.value = null;
  emit('block-drag-outside', null);

  if (state.kind === 'select') {
    if (state.end - state.start >= SNAP_MINUTES) {
      emit('create-block', props.days[state.dayIndex] ?? '', state.start, state.end, event.clientX, event.clientY);
    } else {
      // 单击空白：默认 30 分钟块
      emit(
        'create-block',
        props.days[state.dayIndex] ?? '',
        state.originStart,
        state.originStart + 30,
        event.clientX,
        event.clientY,
      );
    }
    return;
  }

  if (!state.moved) {
    emit('click-block', state.id);
    return;
  }

  if (state.kind === 'move' && !isInsideContent(event.clientX, event.clientY)) {
    emit('block-drop-outside', state.id, event.clientX, event.clientY);
    return;
  }

  if (state.kind === 'move') {
    emit('move-block', state.id, props.days[state.dayIndex] ?? '', state.start, state.end);
  } else {
    emit('resize-block', state.id, state.start, state.end);
  }
}

function onPointerCancel() {
  if (drag.value) {
    emit('block-drag-outside', null);
  }
  drag.value = null;
}

function onKeyDownEscape(event: KeyboardEvent) {
  if (event.key === 'Escape' && drag.value) {
    onPointerCancel();
  }
}

onMounted(() => window.addEventListener('keydown', onKeyDownEscape));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDownEscape));

// 键盘微调：±15 分钟移动，Shift 为主缩放底部
function onBlockKeydown(event: KeyboardEvent, block: TimeBlock) {
  if (!['ArrowUp', 'ArrowDown', 'Enter', ' '].includes(event.key)) {
    return;
  }
  event.preventDefault();
  if (event.key === 'Enter' || event.key === ' ') {
    emit('click-block', block.id);
    return;
  }
  const delta = event.key === 'ArrowUp' ? -SNAP_MINUTES : SNAP_MINUTES;
  if (event.shiftKey) {
    const end = Math.max(block.start + SNAP_MINUTES, Math.min(MINUTES_PER_DAY, block.end + delta));
    emit('resize-block', block.id, block.start, end);
  } else {
    const duration = block.end - block.start;
    const start = Math.min(MINUTES_PER_DAY - duration, Math.max(0, block.start + delta));
    emit('move-block', block.id, block.date, start, start + duration);
  }
}

// ===== 块渲染（拖拽预览合并）=====

const displayBlocks = computed<TimeBlock[]>(() => {
  const state = drag.value;
  if (!state || state.kind === 'select') {
    return props.blocks;
  }
  return props.blocks.map(block => {
    if (block.id !== state.id) {
      return block;
    }
    if (state.kind === 'move') {
      const date = props.days[state.dayIndex] ?? block.date;
      return { ...block, date, start: state.start, end: state.end };
    }
    return { ...block, start: state.start, end: state.end };
  });
});

const blocksByDay = computed<Map<string, TimeBlock[]>>(() => {
  const map = new Map<string, TimeBlock[]>();
  for (const day of props.days) {
    map.set(day, []);
  }
  for (const block of displayBlocks.value) {
    map.get(block.date)?.push(block);
  }
  return map;
});

/** 每天的泳道布局（按日分组，不同日的块互不影响） */
const lanesByDay = computed<Map<string, Map<string, { lane: number; lanes: number }>>>(() => {
  const result = new Map<string, Map<string, { lane: number; lanes: number }>>();
  for (const [day, blocks] of blocksByDay.value) {
    result.set(day, computeLanes(blocks));
  }
  return result;
});

function blockStyle(block: TimeBlock): Record<string, string> {
  const lanes = lanesByDay.value.get(block.date)?.get(block.id) ?? { lane: 0, lanes: 1 };
  const dragging = drag.value?.id === block.id && drag.value.kind !== 'select';
  return {
    top: `${minutesToOffset(block.start)}px`,
    height: `${durationToHeight(block.end - block.start)}px`,
    left: `${(lanes.lane / lanes.lanes) * 100}%`,
    width: `${(1 / lanes.lanes) * 100}%`,
    zIndex: dragging ? '2' : '1',
  };
}

function blockToneClass(block: TimeBlock): string {
  if (block.status === 'done') {
    return 'bg-muted text-muted-foreground';
  }
  if (block.status === 'doing') {
    return 'border-primary/40 bg-primary/[0.08]';
  }
  return 'bg-card';
}

// ===== now 线 =====

const nowLine = computed<{ dayIndex: number; top: number } | null>(() => {
  if (!props.nowMs || !props.todayKey) {
    return null;
  }
  const dayIndex = props.days.indexOf(props.todayKey);
  if (dayIndex === -1) {
    return null;
  }
  const now = new Date(props.nowMs);
  const minutes = now.getHours() * 60 + now.getMinutes();
  if (minutes >= MINUTES_PER_DAY) {
    return null;
  }
  return { dayIndex, top: minutesToOffset(minutes) };
});

// ===== 左栏 HTML5 拖入 =====

const externalPreview = ref<{ dayIndex: number; start: number } | null>(null);

function onDragOver(event: DragEvent) {
  if (!props.externalDragActive) {
    return;
  }
  event.preventDefault();
  const point = pointToMinutes(event.clientX, event.clientY);
  if (point) {
    externalPreview.value = { dayIndex: point.dayIndex, start: snapMinutes(point.minutes) };
  }
}

function onDragLeave(event: DragEvent) {
  if (!contentEl.value?.contains(event.relatedTarget as Node | null)) {
    externalPreview.value = null;
  }
}

function onDrop(event: DragEvent) {
  const preview = externalPreview.value;
  externalPreview.value = null;
  if (!preview) {
    return;
  }
  event.preventDefault();
  emit('external-drop', props.days[preview.dayIndex] ?? '', preview.start);
}

function externalPreviewStyle(): Record<string, string> {
  const preview = externalPreview.value;
  if (!preview) {
    return { display: 'none' };
  }
  return {
    top: `${minutesToOffset(preview.start)}px`,
    height: `${durationToHeight(props.externalDuration)}px`,
    left: `${(preview.dayIndex / dayCount.value) * 100}%`,
    width: `${(1 / dayCount.value) * 100}%`,
  };
}

// 初始滚到 07:30，让工作时段落在视口上部
onMounted(() => {
  if (scrollEl.value) {
    scrollEl.value.scrollTop = minutesToOffset(7 * 60 + 30);
  }
});

function timeRangeText(block: TimeBlock): string {
  return `${formatMinutes(block.start)}–${formatMinutes(block.end)}`;
}

const dragTooltip = computed<{ text: string; x: number; y: number } | null>(() => {
  const state = drag.value;
  if (!state || state.kind === 'select' || !state.moved) {
    return null;
  }
  return {
    text: `${formatMinutes(state.start)}–${formatMinutes(state.end)}`,
    x: dragPointer.value.x,
    y: dragPointer.value.y,
  };
});

const selectionStyle = computed<Record<string, string> | null>(() => {
  const state = drag.value;
  if (!state || state.kind !== 'select' || state.end - state.start < SNAP_MINUTES) {
    return null;
  }
  return {
    top: `${minutesToOffset(state.start)}px`,
    height: `${durationToHeight(state.end - state.start)}px`,
    left: `${(state.dayIndex / dayCount.value) * 100}%`,
    width: `${(1 / dayCount.value) * 100}%`,
  };
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-x-auto rounded-xl border bg-background" isolation="isolate">
    <div class="flex min-h-0 flex-1 flex-col" :style="{ minWidth: minWidth }">
      <!-- 列头 -->
      <div class="flex shrink-0 border-b border-border">
        <div class="shrink-0 border-r border-border" :style="{ width: `${GUTTER_WIDTH}px` }" />
        <div
          v-for="header of dayHeaders"
          :key="header.key"
          class="flex flex-1 items-center justify-center gap-1.5 border-l border-border py-2 first:border-l-0"
          :class="header.isToday ? 'bg-primary/[0.04]' : ''"
        >
          <span class="text-xs" :class="header.isToday ? 'font-medium text-primary' : 'text-muted-foreground'">
            {{ header.weekday }}
          </span>
          <span
            class="text-xs tabular-nums"
            :class="header.isToday ? 'font-medium text-primary' : 'text-muted-foreground'"
          >
            {{ header.dayOfMonth }}
          </span>
          <span
            v-if="header.isToday"
            class="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary"
          >
            今天
          </span>
        </div>
      </div>

      <!-- 网格主体 -->
      <div ref="scrollEl" class="min-h-0 flex-1 overflow-y-auto">
        <div
          ref="contentEl"
          class="relative w-full select-none"
          :style="{ height: `${GRID_HEIGHT}px`, touchAction: 'none' }"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerCancel"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
        >
          <!-- 小时/半小时刻度线（右移让出时间 gutter） -->
          <div class="absolute inset-y-0 right-0" :style="{ left: `${GUTTER_WIDTH}px` }">
            <div
              v-for="top of hourLines"
              :key="`h${top}`"
              class="absolute inset-x-0 border-t border-border/60"
              :style="{ top: `${top}px` }"
            />
            <div
              v-for="top of halfHourLines"
              :key="`m${top}`"
              class="absolute inset-x-0 border-t border-border/25"
              :style="{ top: `${top}px` }"
            />

            <!-- 今日列底色 -->
            <div
              v-if="todayKey && days.includes(todayKey)"
              class="absolute inset-y-0 bg-primary/[0.03]"
              :style="{
                left: `${(days.indexOf(todayKey) / dayCount) * 100}%`,
                width: `${(1 / dayCount) * 100}%`,
              }"
            />

            <!-- 时间块 -->
            <div
              v-for="block of displayBlocks"
              :key="block.id"
              class="group absolute cursor-grab p-px outline-none active:cursor-grabbing focus-visible:ring-2 focus-visible:ring-ring"
              :class="[
                blockToneClass(block),
                block.status === 'done' ? 'border border-transparent' : 'border',
                block.overdue ? 'border-l-2 border-l-destructive' : '',
                drag?.id === block.id && drag.kind !== 'select' ? 'shadow-md' : '',
              ]"
              :style="blockStyle(block)"
              :tabindex="0"
              role="button"
              :aria-label="`${block.title}，${timeRangeText(block)}`"
              :data-block-id="block.id"
              @keydown="onBlockKeydown($event, block)"
            >
              <div
                class="pointer-events-none flex size-full flex-col gap-0.5 overflow-hidden rounded-md px-2 py-1"
                :class="block.status === 'done' ? '' : 'bg-card/80'"
              >
                <p
                  class="truncate text-xs font-medium leading-4"
                  :class="[
                    block.end - block.start <= 30 ? 'text-[10px] leading-3' : '',
                    block.status === 'done' ? 'line-through' : '',
                  ]"
                >
                  {{ block.title }}
                </p>
                <p v-if="block.end - block.start > 45" class="text-[10px] leading-3 tabular-nums text-muted-foreground">
                  {{ timeRangeText(block) }}
                </p>
              </div>
              <!-- 完成切换（hover 显示） -->
              <button
                type="button"
                class="absolute left-1 top-1 hidden rounded-full bg-background/90 p-0.5 text-muted-foreground shadow-sm group-hover:block hover:text-primary focus-visible:block"
                :aria-label="block.status === 'done' ? '标记为待办' : '标记为已完成'"
                @pointerdown.stop
                @click.stop="emit('toggle-block', block.id)"
              >
                <svg
                  class="size-3"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path v-if="block.status === 'done'" d="M20 6 9 17l-5-5" />
                  <circle v-else cx="12" cy="12" r="9" />
                </svg>
              </button>
              <!-- 缩放热区 -->
              <div
                data-resize="top"
                class="absolute inset-x-0 top-0 h-1.5 cursor-ns-resize"
                :class="drag?.id === block.id ? '' : 'opacity-0 hover:opacity-100'"
              />
              <div
                data-resize="bottom"
                class="absolute inset-x-0 bottom-0 h-1.5 cursor-ns-resize"
                :class="drag?.id === block.id ? '' : 'opacity-0 hover:opacity-100'"
              />
            </div>

            <!-- now 线 -->
            <div
              v-if="nowLine"
              class="pointer-events-none absolute inset-y-0"
              :style="{ left: `${(nowLine.dayIndex / dayCount) * 100}%`, width: `${(1 / dayCount) * 100}%` }"
            >
              <div class="relative inset-x-0 top-0 h-0" :style="{ marginTop: `${nowLine.top}px` }">
                <span class="absolute -left-1 top-1/2 size-1.5 -translate-y-1/2 rounded-full bg-primary" />
                <div class="border-t-2 border-primary" />
              </div>
            </div>

            <!-- 空态提示 -->
            <div
              v-if="blocks.length === 0 && !drag"
              class="pointer-events-none absolute inset-x-0 top-1/3 flex justify-center"
            >
              <p class="rounded-full bg-muted/60 px-4 py-1.5 text-xs text-muted-foreground">
                把左侧任务拖进时间格，或在空白处拖选一段直接新建
              </p>
            </div>

            <!-- 框选预览 -->
            <div
              v-if="selectionStyle"
              class="pointer-events-none absolute rounded-md border border-primary/50 bg-primary/10"
              :style="selectionStyle"
            />

            <!-- 左栏拖入预览 -->
            <div
              v-if="externalPreview"
              class="pointer-events-none absolute rounded-md border border-primary/60 bg-primary/15"
              :style="externalPreviewStyle()"
            >
              <p class="truncate px-2 py-1 text-[10px] font-medium text-primary">
                {{ formatMinutes(externalPreview.start) }} 起约 {{ Math.round(externalDuration / 15) * 15 }} 分钟
              </p>
            </div>
          </div>

          <!-- 时间 gutter 标签 -->
          <div class="absolute inset-y-0 left-0" :style="{ width: `${GUTTER_WIDTH}px` }">
            <div
              v-for="hour in 24"
              :key="hour"
              class="absolute right-1.5 -translate-y-1/2 text-[10px] tabular-nums text-muted-foreground"
              :style="{ top: `${minutesToOffset((hour - 1) * 60)}px` }"
            >
              {{ formatMinutes((hour - 1) * 60) }}
            </div>
          </div>

          <!-- 拖拽中的时间 tooltip -->
          <div
            v-if="dragTooltip"
            class="pointer-events-none fixed z-50 -translate-y-8 rounded-md bg-foreground px-2 py-1 text-[10px] font-medium tabular-nums text-background shadow-md"
            :style="{ left: `${dragTooltip.x}px`, top: `${dragTooltip.y}px` }"
          >
            {{ dragTooltip.text }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

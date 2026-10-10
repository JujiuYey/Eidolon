<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { RotateCcw, X, ZoomIn, ZoomOut } from 'lucide-vue-next';

/**
 * mermaid 图表全屏预览:拖动平移、滚轮缩放(锚定鼠标位置)、
 * 单击空白关闭(与拖动按位移区分),底部控制条提供按钮缩放与重置。
 * 复用节点视图渲染好的 svg,矢量缩放不失真。
 */
const props = defineProps<{ svg: string }>();
const emit = defineEmits<{ (e: 'close'): void }>();

const stage = ref<HTMLElement | null>(null);
const natW = ref(0);
const natH = ref(0);
const zoom = ref(1);
const fitZoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const dragging = ref(false);

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 6;

let pointerId: number | null = null;
let downX = 0;
let downY = 0;
let lastX = 0;
let lastY = 0;

function clampZoom(value: number) {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value));
}

// 锚定鼠标缩放:光标指向的图上那点,缩放前后保持在光标下
function setZoom(next: number, anchor?: { x: number; y: number }) {
  const z0 = zoom.value;
  const z1 = clampZoom(next);
  if (z1 === z0) {
    return;
  }
  if (anchor && stage.value) {
    const rect = stage.value.getBoundingClientRect();
    const sx = anchor.x - rect.left - rect.width / 2;
    const sy = anchor.y - rect.top - rect.height / 2;
    panX.value = sx - (sx - panX.value) * (z1 / z0);
    panY.value = sy - (sy - panY.value) * (z1 / z0);
  }
  zoom.value = z1;
}

function onWheel(event: WheelEvent) {
  setZoom(zoom.value * Math.exp(-event.deltaY * 0.0015), { x: event.clientX, y: event.clientY });
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  pointerId = event.pointerId;
  downX = lastX = event.clientX;
  downY = lastY = event.clientY;
  dragging.value = true;
  stage.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (!dragging.value || event.pointerId !== pointerId) {
    return;
  }
  panX.value += event.clientX - lastX;
  panY.value += event.clientY - lastY;
  lastX = event.clientX;
  lastY = event.clientY;
}

function onPointerUp(event: PointerEvent) {
  if (event.pointerId !== pointerId) {
    return;
  }
  const moved = Math.hypot(event.clientX - downX, event.clientY - downY);
  dragging.value = false;
  pointerId = null;
  // 没怎么动就当单击:关闭预览
  if (moved < 3) {
    emit('close');
  }
}

function reset() {
  zoom.value = fitZoom.value;
  panX.value = 0;
  panY.value = 0;
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    emit('close');
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeydown);
  await nextTick();
  const svg = stage.value?.querySelector('svg');
  if (svg) {
    const viewBox = svg.getAttribute('viewBox');
    if (viewBox) {
      const parts = viewBox.split(/[\s,]+/).map(Number);
      const width = parts[2];
      const height = parts[3];
      if (width && height && Number.isFinite(width) && Number.isFinite(height)) {
        natW.value = width;
        natH.value = height;
      }
    }
    if (!natW.value || !natH.value) {
      const rect = svg.getBoundingClientRect();
      natW.value = rect.width;
      natH.value = rect.height;
    }
  }
  const availW = (stage.value?.clientWidth ?? window.innerWidth) - 48;
  fitZoom.value = natW.value > 0 ? Math.min(1, availW / natW.value) : 1;
  zoom.value = fitZoom.value;
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      class="fixed inset-0 z-50 flex flex-col bg-black/70"
      role="dialog"
      aria-modal="true"
      aria-label="Mermaid 图表预览"
    >
      <div class="flex shrink-0 items-center justify-between px-4 py-3">
        <p class="text-sm text-white/80">
          Mermaid 图表
        </p>
        <button
          type="button"
          class="inline-flex h-8 w-8 items-center justify-center rounded-md text-white/80 transition-colors duration-100 hover:bg-white/10 hover:text-white"
          aria-label="关闭预览"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <div
        ref="stage"
        class="zoom-stage relative min-h-0 flex-1 select-none overflow-hidden"
        :class="dragging ? 'cursor-grabbing' : 'cursor-grab'"
        @wheel.prevent="onWheel"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
      >
        <div
          class="zoom-canvas absolute left-1/2 top-1/2 rounded-lg border bg-background"
          :style="{
            width: `${Math.round(natW * zoom)}px`,
            height: `${Math.round(natH * zoom)}px`,
            transform: `translate(-50%, -50%) translate(${Math.round(panX)}px, ${Math.round(panY)}px)`,
          }"
          v-html="props.svg"
        />
      </div>

      <div class="flex shrink-0 justify-center pb-5">
        <div class="flex items-center gap-1 rounded-full border bg-background px-2 py-1 shadow-lg">
          <button
            type="button"
            class="inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors duration-100 hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
            aria-label="缩小"
            :disabled="zoom <= MIN_ZOOM"
            @click="setZoom(zoom - 0.25)"
          >
            <ZoomOut class="h-4 w-4" />
          </button>
          <span class="min-w-[3.5rem] text-center text-xs text-muted-foreground tabular-nums">
            {{ Math.round(zoom * 100) }}%
          </span>
          <button
            type="button"
            class="inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors duration-100 hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
            aria-label="放大"
            :disabled="zoom >= MAX_ZOOM"
            @click="setZoom(zoom + 0.25)"
          >
            <ZoomIn class="h-4 w-4" />
          </button>
          <span class="mx-1 h-4 w-px bg-border" />
          <button
            type="button"
            class="inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors duration-100 hover:bg-accent hover:text-foreground"
            aria-label="重置缩放"
            @click="reset"
          >
            <RotateCcw class="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.zoom-stage {
  touch-action: none;
}

.zoom-canvas :deep(svg) {
  width: 100%;
  height: 100%;
  max-width: none;
  display: block;
  border-radius: inherit;
}
</style>

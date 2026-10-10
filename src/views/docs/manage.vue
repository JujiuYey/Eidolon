<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog';
import { FolderOpen, FolderPlus, Library, Trash2 } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import { useDocsStore } from '@/stores/docs';
import type { DocsRoot } from '@/types/docs';
import { getErrorMessage } from '@/utils/helpers';

import { docsRootLabels } from './utils/roots';

const store = useDocsStore();
const router = useRouter();

const labels = computed(() => docsRootLabels(store.roots));
const isPicking = ref(false);

onMounted(() => {
  void store.initialize().catch(error => toast.error(getErrorMessage(error, '文档库加载失败')));
});

// ===== 添加（可一次多选文件夹） =====
async function pickRoots() {
  isPicking.value = true;
  try {
    const selected = await open({
      directory: true,
      multiple: true,
      title: '添加文档库目录',
    });
    if (!selected) {
      return;
    }
    const picked = (Array.isArray(selected) ? selected : [selected]).filter(Boolean);
    if (picked.length === 0) {
      return;
    }

    const known = new Set(store.roots.map(root => root.path.replace(/\/+$/, '')));
    const additions = picked.filter(path => !known.has(path.replace(/\/+$/, ''))).map(path => ({ id: '', path }));
    if (additions.length === 0) {
      toast.info('所选文件夹已经在文档库列表里了');
      return;
    }

    const before = store.roots.length;
    const saved = await store.saveRoots([...store.roots, ...additions]);
    const added = saved.length - before;
    toast.success(added > 1 ? `已添加 ${added} 个文档库` : '已添加文档库');

    // 从零到一：第一次添加后直接进入该库
    if (before === 0 && saved.length > 0) {
      const first = saved[0];
      if (first) {
        await router.push({ name: 'docs-root', params: { rootId: first.id } });
      }
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '添加文档库失败'));
  } finally {
    isPicking.value = false;
  }
}

// ===== 移除（只移出列表，不动磁盘文件） =====
const removeOpen = ref(false);
const pendingRemove = ref<DocsRoot | null>(null);

function askRemove(root: DocsRoot) {
  pendingRemove.value = root;
  removeOpen.value = true;
}

async function confirmRemove() {
  const root = pendingRemove.value;
  removeOpen.value = false;
  pendingRemove.value = null;
  if (!root) {
    return;
  }

  try {
    await store.saveRoots(store.roots.filter(item => item.id !== root.id));
    toast.success('已移除文档库（磁盘文件不受影响）');
  } catch (error) {
    toast.error(getErrorMessage(error, '移除文档库失败'));
  }
}
</script>

<template>
  <div class="w-full h-full flex flex-col overflow-hidden p-6">
    <div class="mb-6 shrink-0 flex flex-col gap-4 border-b pb-5 lg:flex-row lg:items-end lg:justify-between">
      <div>
        <h1 class="flex items-center gap-2 text-2xl font-semibold tracking-tight text-foreground">
          <Library class="size-6 text-primary" />
          管理目录
        </h1>
        <p class="mt-2 text-sm text-muted-foreground">
          文档库是磁盘上的普通文件夹，只显示其中的 .md 文件。移除文档库只是从应用里移出，不会删除磁盘上的文件。
        </p>
      </div>

      <Button :disabled="isPicking" @click="pickRoots">
        <Spinner v-if="isPicking" class="size-4" />
        <FolderPlus v-else class="size-4" />
        添加文档库
      </Button>
    </div>

    <!-- 空态 -->
    <div
      v-if="store.roots.length === 0"
      class="flex flex-1 flex-col items-center justify-center gap-4 rounded-xl border bg-card text-center"
    >
      <div class="flex size-11 items-center justify-center rounded-full border">
        <FolderOpen class="size-5 text-primary" />
      </div>
      <div>
        <p class="text-sm font-semibold">还没有文档库</p>
        <p class="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
          选一个存放 .md 文件的文件夹作为文档库，里面的文档会出现在侧边栏和文档工作区。
          文档始终是磁盘上的普通文件，随时可以用其他工具打开。
        </p>
      </div>
      <Button :disabled="isPicking" @click="pickRoots">
        <FolderPlus class="size-4" />
        添加第一个文档库
      </Button>
    </div>

    <!-- 库列表 -->
    <div v-else class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
      <div
        v-for="root of store.roots"
        :key="root.id"
        class="flex items-center justify-between gap-4 rounded-xl border bg-card px-4 py-3"
      >
        <div class="flex min-w-0 items-center gap-3">
          <div class="flex size-9 shrink-0 items-center justify-center rounded-lg border bg-muted/40">
            <FolderOpen class="size-4 text-primary" />
          </div>
          <div class="min-w-0">
            <p class="truncate text-sm font-medium">{{ labels.get(root.id) ?? root.id }}</p>
            <p class="truncate text-xs text-muted-foreground">{{ root.path }}</p>
          </div>
        </div>
        <Button aria-label="移除文档库" class="shrink-0" size="icon" variant="ghost" @click="askRemove(root)">
          <Trash2 class="size-4 text-muted-foreground" />
        </Button>
      </div>
    </div>

    <SagConfirm
      v-model:open="removeOpen"
      :description="
        pendingRemove
          ? `把「${labels.get(pendingRemove.id) ?? pendingRemove.id}」从应用里移出？磁盘上的文件和文件夹都会原样保留。`
          : ''
      "
      title="移除文档库"
      type="destructive"
      @confirm="confirmRemove"
    />
  </div>
</template>

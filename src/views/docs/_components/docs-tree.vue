<script setup lang="ts">
import {
  ChevronRight,
  ClipboardCopy,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  MoreHorizontal,
  Pencil,
  Plus,
  Settings2,
  SquareArrowOutUpRight,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import { Button } from '@/components/ui/button';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import type { DocsEntry } from '@/types/docs';

import { buildDocsTree, firstLevelDirPaths, flattenTree } from '../utils/tree';

interface Props {
  entries: DocsEntry[];
  activePath: string;
  /** 文档库 id：切换它才重置展开状态 */
  rootId: string;
  isLoading: boolean;
  /** 树头部标题：当前文档库的名字 */
  title: string;
}

interface Emits {
  (e: 'select', path: string): void;
  (e: 'create-file', dirPath: string): void;
  (e: 'create-dir', parentPath: string): void;
  (e: 'rename', entry: DocsEntry): void;
  (e: 'remove', entry: DocsEntry): void;
  (e: 'reveal', entry: DocsEntry): void;
  (e: 'copy-path', entry: DocsEntry): void;
  (e: 'manage'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const expanded = ref(new Set<string>());

/**
 * 只有换文档库才重置展开状态。
 * 之前监听 entries，而 entries 每次重载都是新数组引用，结果新建/重命名/删除
 * 都会把用户展开的层级折回根层——新建在子目录里的文件因此"看不见"。
 */
watch(
  () => props.rootId,
  () => {
    expanded.value = new Set(firstLevelDirPaths(props.entries));
  },
  { immediate: true },
);

// 条目刷新时保留已有展开，并让新建的根层目录自动可见
watch(
  () => props.entries,
  entries => {
    expanded.value = new Set([...expanded.value, ...firstLevelDirPaths(entries)]);
  },
);

const rows = computed(() => flattenTree(buildDocsTree(props.entries), expanded.value));

function toggleDir(path: string) {
  const next = new Set(expanded.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  expanded.value = next;
}

function rowPadding(depth: number) {
  return { paddingLeft: `${8 + depth * 14}px` };
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col rounded-xl border bg-card">
    <!-- 空白处右键：文档库级别的操作，与条目右键菜单区分开 -->
    <ContextMenu>
      <ContextMenuTrigger as-child>
        <div class="flex min-h-0 flex-1 flex-col">
          <div class="flex items-center justify-between gap-2 px-4 py-3">
            <h2 class="flex items-center gap-2 text-sm font-semibold">
              <FolderOpen class="h-4 w-4 text-primary" />
              {{ title }}
            </h2>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button aria-label="文档库操作" size="icon" variant="ghost">
                  <MoreHorizontal class="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem @click="emit('create-file', '')">
                  <Plus class="mr-2 h-4 w-4" />
                  新建
                </DropdownMenuItem>
                <DropdownMenuItem @click="emit('create-dir', '')">
                  <FolderPlus class="mr-2 h-4 w-4" />
                  新建文件夹
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem @click="emit('manage')">
                  <Settings2 class="mr-2 h-4 w-4" />
                  管理文档库
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          <ScrollArea class="min-h-0 flex-1">
            <div class="space-y-0.5 px-2 pb-2">
              <template v-if="isLoading">
                <Skeleton v-for="index of 5" :key="index" class="mx-2 h-8" />
              </template>

              <p v-else-if="rows.length === 0" class="px-2 py-6 text-center text-xs text-muted-foreground">
                文档库是空的，点「新建」写下第一篇。
              </p>

              <template v-else>
                <ContextMenu v-for="row of rows" :key="row.entry.path">
                  <ContextMenuTrigger as-child>
                    <div
                      class="flex cursor-default items-center rounded-lg transition-colors hover:bg-muted/60"
                      :class="row.entry.path === activePath && !row.entry.is_dir ? 'bg-muted' : ''"
                      :style="rowPadding(row.depth)"
                    >
                      <button
                        type="button"
                        class="flex min-w-0 flex-1 items-center gap-1.5 py-1.5 pr-1 text-left"
                        @click="row.entry.is_dir ? toggleDir(row.entry.path) : emit('select', row.entry.path)"
                      >
                        <ChevronRight
                          v-if="row.entry.is_dir"
                          class="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform"
                          :class="expanded.has(row.entry.path) ? 'rotate-90' : ''"
                        />
                        <span v-else class="w-3.5 shrink-0" />
                        <component
                          :is="row.entry.is_dir ? (expanded.has(row.entry.path) ? FolderOpen : Folder) : FileText"
                          class="h-4 w-4 shrink-0"
                          :class="row.entry.is_dir ? 'text-primary/80' : 'text-muted-foreground'"
                        />
                        <span class="truncate text-sm">{{ row.entry.name }}</span>
                      </button>
                    </div>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem v-if="row.entry.is_dir" @click="emit('create-file', row.entry.path)">
                      <Plus class="mr-2 h-4 w-4" />
                      在此新建文档
                    </ContextMenuItem>
                    <ContextMenuItem v-if="row.entry.is_dir" @click="emit('create-dir', row.entry.path)">
                      <FolderPlus class="mr-2 h-4 w-4" />
                      在此新建文件夹
                    </ContextMenuItem>
                    <ContextMenuSeparator v-if="row.entry.is_dir" />
                    <ContextMenuItem @click="emit('reveal', row.entry)">
                      <SquareArrowOutUpRight class="mr-2 h-4 w-4" />
                      在访达中显示
                    </ContextMenuItem>
                    <ContextMenuItem @click="emit('copy-path', row.entry)">
                      <ClipboardCopy class="mr-2 h-4 w-4" />
                      复制路径
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem @click="emit('rename', row.entry)">
                      <Pencil class="mr-2 h-4 w-4" />
                      重命名
                    </ContextMenuItem>
                    <ContextMenuItem class="text-destructive focus:text-destructive" @click="emit('remove', row.entry)">
                      <Trash2 class="mr-2 h-4 w-4" />
                      删除
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              </template>
            </div>
          </ScrollArea>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem @click="emit('create-file', '')">
          <Plus class="mr-2 h-4 w-4" />
          新建文档
        </ContextMenuItem>
        <ContextMenuItem @click="emit('create-dir', '')">
          <FolderPlus class="mr-2 h-4 w-4" />
          新建文件夹
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem @click="emit('manage')">
          <Settings2 class="mr-2 h-4 w-4" />
          管理文档库
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  </div>
</template>

<script setup lang="ts">
import type { DocsEntry } from '@/types/docs';
import {
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  MoreHorizontal,
  Pencil,
  Plus,
  Trash2,
} from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { buildDocsTree, flattenTree } from '../utils/tree';

interface Props {
  entries: DocsEntry[];
  activePath: string;
  isLoading: boolean;
}

interface Emits {
  (e: 'select', path: string): void;
  (e: 'create-file', dirPath: string): void;
  (e: 'create-dir', parentPath: string): void;
  (e: 'rename', entry: DocsEntry): void;
  (e: 'remove', entry: DocsEntry): void;
  (e: 'change-root'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const expanded = ref(new Set<string>());

// 换根目录 / 重载后重置为默认展开（根层目录）
watch(
  () => props.entries,
  () => {
    expanded.value = new Set(
      props.entries
        .filter(entry => entry.is_dir && !entry.path.includes('/'))
        .map(entry => entry.path),
    );
  },
  { immediate: true },
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
    <div class="flex items-center justify-between gap-2 px-4 py-3">
      <h2 class="flex items-center gap-2 text-sm font-semibold">
        <FolderOpen class="h-4 w-4 text-primary" />
        文档
      </h2>
      <div class="flex items-center gap-1">
        <Button
          size="sm"
          variant="outline"
          @click="emit('create-file', '')"
        >
          <Plus class="h-4 w-4" />
          新建
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              aria-label="文档库操作"
              size="icon"
              variant="ghost"
            >
              <MoreHorizontal class="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem @click="emit('create-dir', '')">
              <FolderPlus class="mr-2 h-4 w-4" />
              新建文件夹
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem @click="emit('change-root')">
              <Pencil class="mr-2 h-4 w-4" />
              更改文档库目录
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-0.5 px-2 pb-2">
        <template v-if="isLoading">
          <Skeleton
            v-for="index of 5"
            :key="index"
            class="mx-2 h-8"
          />
        </template>

        <p
          v-else-if="rows.length === 0"
          class="px-2 py-6 text-center text-xs text-muted-foreground"
        >
          文档库是空的，点「新建」写下第一篇。
        </p>

        <template v-else>
          <div
            v-for="row of rows"
            :key="row.entry.path"
            class="group flex items-center gap-1 rounded-lg pr-1 transition-colors hover:bg-muted/60"
            :class="row.entry.path === activePath && !row.entry.is_dir ? 'bg-muted' : ''"
            :style="rowPadding(row.depth)"
          >
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-1.5 py-1.5 text-left"
              @click="row.entry.is_dir ? toggleDir(row.entry.path) : emit('select', row.entry.path)"
            >
              <ChevronRight
                v-if="row.entry.is_dir"
                class="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform"
                :class="expanded.has(row.entry.path) ? 'rotate-90' : ''"
              />
              <span
                v-else
                class="w-3.5 shrink-0"
              />
              <component
                :is="row.entry.is_dir ? expanded.has(row.entry.path) ? FolderOpen : Folder : FileText"
                class="h-4 w-4 shrink-0"
                :class="row.entry.is_dir ? 'text-primary/80' : 'text-muted-foreground'"
              />
              <span class="truncate text-sm">{{ row.entry.name }}</span>
            </button>

            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button
                  aria-label="条目操作"
                  class="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                  size="icon"
                  variant="ghost"
                >
                  <MoreHorizontal class="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <template v-if="row.entry.is_dir">
                  <DropdownMenuItem @click="emit('create-file', row.entry.path)">
                    <Plus class="mr-2 h-4 w-4" />
                    新建文档
                  </DropdownMenuItem>
                  <DropdownMenuItem @click="emit('create-dir', row.entry.path)">
                    <FolderPlus class="mr-2 h-4 w-4" />
                    新建文件夹
                  </DropdownMenuItem>
                </template>
                <DropdownMenuItem @click="emit('rename', row.entry)">
                  <Pencil class="mr-2 h-4 w-4" />
                  重命名
                </DropdownMenuItem>
                <DropdownMenuItem
                  class="text-destructive focus:text-destructive"
                  @click="emit('remove', row.entry)"
                >
                  <Trash2 class="mr-2 h-4 w-4" />
                  删除
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </template>
      </div>
    </ScrollArea>
  </div>
</template>

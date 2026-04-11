<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { FolderTree, RefreshCw } from 'lucide-vue-next';
import type { ProjectFileEntry } from '@/types/project-files';
import ProjectFileTreeNode from './ProjectFileTreeNode.vue';
import type { ProjectTreeNode } from './ProjectFileTreeNode.vue';

interface Props {
  files: ProjectFileEntry[];
  selectedPath: string | null;
}

interface Emits {
  (e: 'select', path: string): void;
  (e: 'refresh'): void;
}

const props = defineProps<Props>();
defineEmits<Emits>();

const keyword = ref('');
const openPaths = ref<string[]>([]);

const filteredFiles = computed(() => {
  if (!keyword.value.trim()) {
    return props.files;
  }

  const search = keyword.value.trim().toLowerCase();
  return props.files.filter(file => file.path.toLowerCase().includes(search));
});

const shouldForceExpand = computed(() => Boolean(keyword.value.trim()));

const tree = computed<ProjectTreeNode[]>(() => buildTree(filteredFiles.value));

watch(() => props.selectedPath, path => {
  if (!path) {
    return;
  }

  for (const parentPath of getAncestorPaths(path)) {
    if (!openPaths.value.includes(parentPath)) {
      openPaths.value.push(parentPath);
    }
  }
}, { immediate: true });

function togglePath(path: string) {
  openPaths.value = openPaths.value.includes(path)
    ? openPaths.value.filter(item => item !== path)
    : [...openPaths.value, path];
}

function buildTree(files: ProjectFileEntry[]): ProjectTreeNode[] {
  const root: ProjectTreeNode[] = [];

  for (const file of files) {
    const segments = file.path.split('/').filter(Boolean);
    let currentLevel = root;

    segments.forEach((segment, index) => {
      const isLast = index === segments.length - 1;
      const currentPath = segments.slice(0, index + 1).join('/');

      let node = currentLevel.find(item => item.path === currentPath);
      if (!node) {
        node = {
          name: segment,
          path: currentPath,
          type: isLast ? 'file' : 'directory',
          children: isLast ? undefined : [],
        };
        currentLevel.push(node);
        currentLevel.sort((left, right) => {
          if (left.type !== right.type) {
            return left.type === 'directory' ? -1 : 1;
          }
          return left.name.localeCompare(right.name);
        });
      }

      if (!isLast) {
        if (!node.children) {
          node.children = [];
        }
        currentLevel = node.children;
      }
    });
  }

  return root;
}

function getAncestorPaths(path: string) {
  const segments = path.split('/').filter(Boolean);
  return segments.slice(0, -1).map((_, index) => segments.slice(0, index + 1).join('/'));
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="border-b p-4">
      <div class="mb-3 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <FolderTree class="h-4 w-4 text-primary" />
          <span class="font-medium">项目文件</span>
        </div>

        <Button variant="ghost" size="icon" @click="$emit('refresh')">
          <RefreshCw class="h-4 w-4" />
        </Button>
      </div>

      <Input v-model="keyword" placeholder="过滤文件路径" />
      <p class="mt-2 text-xs text-muted-foreground">
        共 {{ files.length }} 个可分析文件
      </p>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-1 p-2">
        <ProjectFileTreeNode
          v-for="node of tree"
          :key="node.path"
          :node="node"
          :selected-path="selectedPath"
          :open-paths="openPaths"
          :force-expand="shouldForceExpand"
          @select="$emit('select', $event)"
          @toggle="togglePath"
        />

        <div v-if="tree.length === 0" class="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
          未找到匹配文件
        </div>
      </div>
    </ScrollArea>
  </div>
</template>

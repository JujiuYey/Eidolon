<script setup lang="ts">
import { computed } from 'vue';
import { ChevronRight, FileCode2, Folder, FolderOpen } from 'lucide-vue-next';
import { cn } from '@/lib/utils';

export interface ProjectTreeNode {
  name: string;
  path: string;
  type: 'directory' | 'file';
  children?: ProjectTreeNode[];
}

interface Props {
  node: ProjectTreeNode;
  level?: number;
  selectedPath: string | null;
  openPaths: string[];
  forceExpand?: boolean;
}

interface Emits {
  (e: 'select', path: string): void;
  (e: 'toggle', path: string): void;
}

const props = withDefaults(defineProps<Props>(), {
  level: 0,
  forceExpand: false,
});

const emit = defineEmits<Emits>();

const isDirectory = computed(() => props.node.type === 'directory');
const isOpen = computed(() => props.forceExpand || props.openPaths.includes(props.node.path));
const isSelected = computed(() => props.selectedPath === props.node.path);
const hasChildren = computed(() => (props.node.children?.length ?? 0) > 0);

function handleDirectoryClick() {
  emit('toggle', props.node.path);
}

function handleFileClick() {
  emit('select', props.node.path);
}
</script>

<template>
  <div>
    <button
      type="button"
      class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent"
      :class="cn(isSelected && 'bg-primary/10 text-primary')"
      :style="{ paddingLeft: `${level * 12 + 8}px` }"
      @click="isDirectory ? handleDirectoryClick() : handleFileClick()"
    >
      <template v-if="isDirectory">
        <ChevronRight class="h-4 w-4 shrink-0 transition-transform" :class="isOpen ? 'rotate-90' : ''" />
        <FolderOpen v-if="isOpen" class="h-4 w-4 shrink-0 text-primary" />
        <Folder v-else class="h-4 w-4 shrink-0 text-muted-foreground" />
      </template>

      <template v-else>
        <span class="inline-flex h-4 w-4 shrink-0 items-center justify-center" />
        <FileCode2 class="h-4 w-4 shrink-0 text-muted-foreground" />
      </template>

      <span class="min-w-0 truncate">{{ node.name }}</span>
      <span v-if="isDirectory && hasChildren" class="ml-auto text-xs text-muted-foreground">
        {{ node.children?.length }}
      </span>
    </button>

    <div v-if="isDirectory && isOpen && hasChildren" class="space-y-0.5">
      <ProjectFileTreeNode
        v-for="child of node.children"
        :key="child.path"
        :node="child"
        :level="level + 1"
        :selected-path="selectedPath"
        :open-paths="openPaths"
        :force-expand="forceExpand"
        @select="$emit('select', $event)"
        @toggle="$emit('toggle', $event)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ChevronDown, ChevronRight, Folder, FolderInput, FolderOpen, MoreHorizontal, Pencil, Plus, PlusCircle, Search, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import type { ApiClientGroup, ApiClientRequest } from '@/types/api-client';
import { buildGroupTree, flattenGroupTree } from './build-group-tree';
import type { GroupTreeNode } from './build-group-tree';

interface Props {
  projectName: string;
  groups: ApiClientGroup[];
  requests: ApiClientRequest[];
  searchKeyword: string;
  activeGroupId: string | null;
  activeRequestId: string | null;
  isLoadingGroups: boolean;
  isLoadingRequests: boolean;
  deletingGroupId: string | null;
  deletingRequestId: string | null;
  renamingGroupId?: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:searchKeyword', value: string): void;
  (e: 'selectGroup', groupId: string): void;
  (e: 'selectRequest', requestId: string): void;
  (e: 'createGroup', payload: { parentGroupId: string | null; parentGroupName: string | null }): void;
  (e: 'createRequest', payload: { groupId: string; groupName: string }): void;
  (e: 'requestRename', groupId: string): void;
  (e: 'deleteGroup', groupId: string): void;
  (e: 'moveRequest', payload: { requestId: string; targetGroupId: string }): void;
  (e: 'deleteRequest', requestId: string): void;
}>();

const expandedGroups = ref<Set<string>>(new Set());
const moveRequestId = ref<string | null>(null);

const trimmedKeyword = computed(() => props.searchKeyword.trim().toLowerCase());

const requestsByGroup = computed(() => {
  const map = new Map<string, ApiClientRequest[]>();
  for (const request of props.requests) {
    const list = map.get(request.groupId) ?? [];
    list.push(request);
    map.set(request.groupId, list);
  }
  for (const list of map.values()) {
    list.sort((a, b) => a.sort - b.sort);
  }
  return map;
});

const filteredRequestsByGroup = computed(() => {
  const keyword = trimmedKeyword.value;
  if (!keyword) {
    return requestsByGroup.value;
  }
  const filtered = new Map<string, ApiClientRequest[]>();
  for (const [groupId, list] of requestsByGroup.value) {
    filtered.set(groupId, list.filter(request => request.name.toLowerCase().includes(keyword)));
  }
  return filtered;
});

const treeRoots = computed<GroupTreeNode[]>(() => buildGroupTree(props.groups));
const treeNodes = computed<GroupTreeNode[]>(() => flattenGroupTree(treeRoots.value));

// Excludes the request's current group so users can't "move" to where it already
// lives. Renders the project as a flat, depth-aware list inside the sub-menu.
function movableGroupTargets(currentGroupId: string): Array<{ id: string; name: string; depth: number }> {
  return treeNodes.value
    .filter(node => node.group.id !== currentGroupId)
    .map(node => ({ id: node.group.id, name: node.group.name, depth: node.depth }));
}

// A group is visible during search if its own name matches, any of its direct
// requests match, or any descendant group has a matching request or name. The
// latter is propagated bottom-up so ancestors stay expanded.
const visibleGroupIds = computed<Set<string>>(() => {
  const keyword = trimmedKeyword.value;
  if (!keyword) {
    return new Set(treeNodes.value.map(node => node.group.id));
  }
  const directMatches = new Set<string>();
  for (const node of treeNodes.value) {
    if (node.group.name.toLowerCase().includes(keyword)) {
      directMatches.add(node.group.id);
    }
  }
  const visible = new Set<string>();
  const sortedByDepth = [...treeNodes.value].sort((a, b) => b.depth - a.depth);
  const groupById = new Map<string, ApiClientGroup>();
  for (const group of props.groups) {
    groupById.set(group.id, group);
  }
  for (const node of sortedByDepth) {
    const hasMatch = directMatches.has(node.group.id)
      || (filteredRequestsByGroup.value.get(node.group.id)?.length ?? 0) > 0;
    if (!hasMatch) {
      continue;
    }
    visible.add(node.group.id);
    let parentId: string | null | undefined = node.group.parentGroupId;
    while (parentId) {
      if (visible.has(parentId)) {
        break;
      }
      visible.add(parentId);
      parentId = groupById.get(parentId)?.parentGroupId;
    }
  }
  return visible;
});

function isGroupVisible(groupId: string): boolean {
  if (!props.searchKeyword.trim()) {
    return true;
  }
  return visibleGroupIds.value.has(groupId);
}

function toggleGroup(groupId: string): void {
  const next = new Set(expandedGroups.value);
  if (next.has(groupId)) {
    next.delete(groupId);
  } else {
    next.add(groupId);
  }
  expandedGroups.value = next;
}

function ensureGroupExpanded(groupId: string): void {
  if (expandedGroups.value.has(groupId)) {
    return;
  }
  const next = new Set(expandedGroups.value);
  next.add(groupId);
  expandedGroups.value = next;
}

function handleGroupClick(group: ApiClientGroup): void {
  toggleGroup(group.id);
  emit('selectGroup', group.id);
}

function handleRequestClick(request: ApiClientRequest, groupId: string): void {
  emit('selectRequest', request.id);
  ensureGroupExpanded(groupId);
}

function handleCreateRoot(event: Event): void {
  event.stopPropagation();
  emit('createGroup', { parentGroupId: null, parentGroupName: null });
}

function handleCreateChild(parentId: string, parentName: string, event: Event): void {
  event.stopPropagation();
  emit('createGroup', { parentGroupId: parentId, parentGroupName: parentName });
}

function handleCreateRequest(groupId: string, groupName: string, event: Event): void {
  event.stopPropagation();
  emit('createRequest', { groupId, groupName });
}

function handleDeleteRequest(requestId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteRequest', requestId);
}

function handleDeleteGroup(groupId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteGroup', groupId);
}

function handleMoveRequest(requestId: string, targetGroupId: string): void {
  if (moveRequestId.value === requestId) {
    return;
  }
  emit('moveRequest', { requestId, targetGroupId });
}

function handleRenameRequest(groupId: string): void {
  emit('requestRename', groupId);
}

function indentStyle(depth: number): { paddingLeft: string } {
  return { paddingLeft: `${(depth - 1) * 16 + 8}px` };
}
</script>

<template>
  <div class="flex h-full flex-col gap-3 bg-card p-3">
    <div class="flex items-start gap-2 rounded-md border bg-background px-3 py-2">
      <FolderOpen class="mt-0.5 size-4 shrink-0 text-primary" />
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium text-foreground">
          {{ projectName }}
        </p>
        <p class="text-[11px] text-muted-foreground">
          当前项目
        </p>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="relative flex-1">
        <Search class="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          :model-value="searchKeyword"
          placeholder="搜索请求"
          class="pl-8"
          @update:model-value="value => emit('update:searchKeyword', String(value ?? ''))"
        />
      </div>
      <button
        type="button"
        class="inline-flex size-9 items-center justify-center rounded-md border bg-background text-foreground transition-colors hover:bg-accent"
        aria-label="新建根分组"
        @click="handleCreateRoot($event)"
      >
        <Plus class="size-4" />
      </button>
    </div>

    <div v-if="isLoadingGroups" class="flex items-center gap-2 text-xs text-muted-foreground">
      <Spinner />
      正在加载分组…
    </div>

    <ScrollArea v-else class="min-h-0 flex-1">
      <ul class="flex flex-col gap-1 pr-2">
        <template v-for="node of treeNodes" :key="node.group.id">
          <li v-show="isGroupVisible(node.group.id)" class="flex flex-col gap-1">
            <div
              class="group flex items-center gap-1 rounded-md py-1 pr-2 hover:bg-accent"
              :class="{ 'bg-accent': node.group.id === activeGroupId }"
              :style="indentStyle(node.depth)"
            >
              <button
                type="button"
                class="flex flex-1 items-center gap-1 text-left text-sm"
                @click="handleGroupClick(node.group)"
              >
                <ChevronDown v-if="expandedGroups.has(node.group.id)" class="size-4 shrink-0 text-muted-foreground" />
                <ChevronRight v-else class="size-4 shrink-0 text-muted-foreground" />
                <Folder class="size-4 shrink-0 text-muted-foreground" />
                <span class="truncate">{{ node.group.name }}</span>
              </button>
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <button
                    type="button"
                    class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-foreground group-hover:opacity-100"
                    :aria-label="`分组操作 ${node.group.name}`"
                    @click.stop
                  >
                    <MoreHorizontal class="size-3.5" />
                  </button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" class="min-w-[160px]">
                  <DropdownMenuItem
                    :disabled="!!renamingGroupId && renamingGroupId === node.group.id"
                    @select="handleCreateChild(node.group.id, node.group.name, $event)"
                  >
                    <PlusCircle class="mr-2 size-3.5" />
                    新建子分组
                  </DropdownMenuItem>
                  <DropdownMenuItem @select="handleCreateRequest(node.group.id, node.group.name, $event)">
                    <Plus class="mr-2 size-3.5" />
                    新建请求
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem @select="handleRenameRequest(node.group.id)">
                    <Pencil class="mr-2 size-3.5" />
                    重命名
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    :disabled="deletingGroupId === node.group.id"
                    class="text-destructive focus:text-destructive"
                    @select="handleDeleteGroup(node.group.id, $event)"
                  >
                    <Trash2 class="mr-2 size-3.5" />
                    删除
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            <ul v-if="expandedGroups.has(node.group.id) && isGroupVisible(node.group.id)" class="flex flex-col gap-1">
              <li v-if="isLoadingRequests" class="flex items-center gap-1 px-2 py-1 text-xs text-muted-foreground">
                <Spinner /> 加载请求…
              </li>

              <li
                v-for="request of filteredRequestsByGroup.get(node.group.id) ?? []"
                v-else
                :key="request.id"
                class="group flex items-center gap-1 rounded-md pr-2 py-1 hover:bg-accent"
                :class="{ 'bg-accent': request.id === activeRequestId }"
                :style="{ paddingLeft: `${node.depth * 16 + 8}px` }"
              >
                <button
                  type="button"
                  class="flex flex-1 items-center gap-2 text-left text-xs"
                  @click="handleRequestClick(request, node.group.id)"
                >
                  <span class="font-mono text-[10px] font-semibold text-primary">{{ request.method }}</span>
                  <span class="truncate">{{ request.name }}</span>
                </button>
                <DropdownMenu>
                  <DropdownMenuTrigger as-child>
                    <button
                      type="button"
                      class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-foreground group-hover:opacity-100 disabled:opacity-50"
                      :disabled="moveRequestId === request.id || deletingRequestId === request.id"
                      :aria-label="`更多操作 ${request.name}`"
                      @click.stop
                    >
                      <MoreHorizontal class="size-3" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" class="min-w-[180px]">
                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger :disabled="moveRequestId === request.id">
                        <FolderInput class="mr-2 size-3.5" />
                        移动到…
                      </DropdownMenuSubTrigger>
                      <DropdownMenuSubContent class="max-h-[280px] min-w-[200px] overflow-y-auto">
                        <DropdownMenuItem
                          v-for="target of movableGroupTargets(request.groupId)"
                          :key="target.id"
                          :disabled="moveRequestId === request.id"
                          @select="handleMoveRequest(request.id, target.id)"
                        >
                          <span :style="indentStyle(target.depth)" class="truncate">{{ target.name }}</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem v-if="movableGroupTargets(request.groupId).length === 0" disabled>
                          暂无可选分组
                        </DropdownMenuItem>
                      </DropdownMenuSubContent>
                    </DropdownMenuSub>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      :disabled="deletingRequestId === request.id"
                      class="text-destructive focus:text-destructive"
                      @select="handleDeleteRequest(request.id, $event)"
                    >
                      <Trash2 class="mr-2 size-3.5" />
                      删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </li>

              <li
                v-if="(filteredRequestsByGroup.get(node.group.id) ?? []).length === 0 && !isLoadingRequests"
                class="pr-2 py-1 text-[11px] text-muted-foreground"
                :style="{ paddingLeft: `${node.depth * 16 + 8}px` }"
              >
                <span v-if="searchKeyword">无匹配请求</span>
                <span v-else>暂无请求</span>
              </li>
            </ul>
          </li>
        </template>

        <li
          v-if="props.groups.length === 0 && !isLoadingGroups"
          class="flex flex-col gap-2 rounded-md border border-dashed bg-muted/20 px-2 py-3 text-center text-[11px] text-muted-foreground"
        >
          当前项目暂无分组
          <button
            type="button"
            class="mx-auto inline-flex items-center gap-1 rounded-md border bg-background px-2 py-1 text-[11px] text-foreground hover:bg-accent"
            @click="handleCreateRoot($event)"
          >
            <Plus class="size-3" />
            新建分组
          </button>
        </li>
      </ul>
    </ScrollArea>
  </div>
</template>

<script setup lang="ts">
import { ChevronDown, ChevronRight, Folder, FolderOpen, Plus, Search, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import type { ApiClientGroup, ApiClientRequest } from '@/types/api-client';

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
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:searchKeyword', value: string): void;
  (e: 'selectGroup', groupId: string): void;
  (e: 'selectRequest', requestId: string): void;
  (e: 'createGroup'): void;
  (e: 'createRequest', payload: { groupId: string; groupName: string }): void;
  (e: 'deleteGroup', groupId: string): void;
  (e: 'deleteRequest', requestId: string): void;
}>();

const expandedGroups = ref<Set<string>>(new Set());

const groupsSorted = computed(() => [...props.groups].sort((a, b) => a.sort - b.sort));
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

const trimmedKeyword = computed(() => props.searchKeyword.trim().toLowerCase());

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

function toggleGroup(groupId: string): void {
  if (expandedGroups.value.has(groupId)) {
    expandedGroups.value.delete(groupId);
  } else {
    expandedGroups.value.add(groupId);
  }
  expandedGroups.value = new Set(expandedGroups.value);
}

function ensureGroupExpanded(groupId: string): void {
  if (!expandedGroups.value.has(groupId)) {
    expandedGroups.value.add(groupId);
    expandedGroups.value = new Set(expandedGroups.value);
  }
}

function selectGroup(groupId: string): void {
  ensureGroupExpanded(groupId);
  emit('selectGroup', groupId);
}

function selectRequest(requestId: string, groupId: string): void {
  emit('selectRequest', requestId);
  ensureGroupExpanded(groupId);
}

function handleDeleteGroup(groupId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteGroup', groupId);
}

function handleDeleteRequest(requestId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteRequest', requestId);
}

function handleCreateGroup(event: Event): void {
  event.stopPropagation();
  emit('createGroup');
}

function handleCreateRequest(groupId: string, groupName: string, event: Event): void {
  event.stopPropagation();
  emit('createRequest', { groupId, groupName });
}
</script>

<template>
  <div class="flex h-full flex-col gap-3 border-r bg-card p-3">
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
        aria-label="新建分组"
        @click="handleCreateGroup($event)"
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
        <li
          v-for="group of groupsSorted"
          v-show="!searchKeyword || (filteredRequestsByGroup.get(group.id)?.length ?? 0) > 0 || group.name.toLowerCase().includes(trimmedKeyword)"
          :key="group.id"
          class="flex flex-col gap-1"
        >
          <div
            class="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-accent"
            :class="{ 'bg-accent': group.id === activeGroupId }"
          >
            <button
              type="button"
              class="flex flex-1 items-center gap-1 text-left text-sm"
              @click="toggleGroup(group.id); selectGroup(group.id)"
            >
              <ChevronDown v-if="expandedGroups.has(group.id)" class="size-4 text-muted-foreground" />
              <ChevronRight v-else class="size-4 text-muted-foreground" />
              <Folder class="size-4 text-muted-foreground" />
              <span class="truncate">{{ group.name }}</span>
            </button>
            <button
              type="button"
              class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-foreground group-hover:opacity-100"
              :aria-label="`新建请求 ${group.name}`"
              @click="handleCreateRequest(group.id, group.name, $event)"
            >
              <Plus class="size-3.5" />
            </button>
            <button
              type="button"
              class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-destructive group-hover:opacity-100 disabled:opacity-50"
              :disabled="deletingGroupId === group.id"
              :aria-label="`删除分组 ${group.name}`"
              @click="handleDeleteGroup(group.id, $event)"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>

          <ul v-if="expandedGroups.has(group.id)" class="flex flex-col gap-1 pl-4">
            <li v-if="isLoadingRequests" class="flex items-center gap-1 px-2 py-1 text-xs text-muted-foreground">
              <Spinner /> 加载请求…
            </li>

            <li
              v-for="request of filteredRequestsByGroup.get(group.id) ?? []"
              v-else
              :key="request.id"
              class="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-accent"
              :class="{ 'bg-accent': request.id === activeRequestId }"
            >
              <button
                type="button"
                class="flex flex-1 items-center gap-2 text-left text-xs"
                @click="selectRequest(request.id, group.id)"
              >
                <span class="font-mono text-[10px] font-semibold text-primary">{{ request.method }}</span>
                <span class="truncate">{{ request.name }}</span>
              </button>
              <button
                type="button"
                class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-destructive group-hover:opacity-100 disabled:opacity-50"
                :disabled="deletingRequestId === request.id"
                :aria-label="`删除请求 ${request.name}`"
                @click="handleDeleteRequest(request.id, $event)"
              >
                <Trash2 class="size-3" />
              </button>
            </li>

            <li
              v-if="(filteredRequestsByGroup.get(group.id) ?? []).length === 0 && !isLoadingRequests"
              class="px-2 py-1 text-[11px] text-muted-foreground"
            >
              <span v-if="searchKeyword">无匹配请求</span>
              <span v-else>暂无请求</span>
            </li>
          </ul>
        </li>

        <li
          v-if="groupsSorted.length === 0 && !isLoadingGroups"
          class="flex flex-col gap-2 rounded-md border border-dashed bg-muted/20 px-2 py-3 text-center text-[11px] text-muted-foreground"
        >
          当前项目暂无分组
          <button
            type="button"
            class="mx-auto inline-flex items-center gap-1 rounded-md border bg-background px-2 py-1 text-[11px] text-foreground hover:bg-accent"
            @click="handleCreateGroup($event)"
          >
            <Plus class="size-3" />
            新建分组
          </button>
        </li>
      </ul>
    </ScrollArea>
  </div>
</template>

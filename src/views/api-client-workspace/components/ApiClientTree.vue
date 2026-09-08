<script setup lang="ts">
import { ChevronDown, ChevronRight, Folder, FolderOpen, Plus, Search, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Spinner } from '@/components/ui/spinner';
import type { ApiClientGroup, ApiClientProject, ApiClientRequest } from '@/types/api-client';

interface Props {
  projects: ApiClientProject[];
  groups: ApiClientGroup[];
  requests: ApiClientRequest[];
  searchKeyword: string;
  activeProjectId: string | null;
  activeGroupId: string | null;
  activeRequestId: string | null;
  isLoadingProjects: boolean;
  isLoadingGroups: boolean;
  isLoadingRequests: boolean;
  deletingProjectId: string | null;
  deletingGroupId: string | null;
  deletingRequestId: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:searchKeyword', value: string): void;
  (e: 'selectProject', projectId: string): void;
  (e: 'selectGroup', groupId: string): void;
  (e: 'selectRequest', requestId: string): void;
  (e: 'createProject'): void;
  (e: 'createGroup', groupId: string | null): void;
  (e: 'createRequest', payload: { groupId: string; groupName: string }): void;
  (e: 'deleteProject', projectId: string): void;
  (e: 'deleteGroup', groupId: string): void;
  (e: 'deleteRequest', requestId: string): void;
}>();

const expandedProjects = ref<Set<string>>(new Set());
const expandedGroups = ref<Set<string>>(new Set());

const projectsSorted = computed(() => [...props.projects].sort((a, b) => a.sort - b.sort));
const groupsByProject = computed(() => {
  const map = new Map<string, ApiClientGroup[]>();
  for (const group of props.groups) {
    const list = map.get(group.projectId) ?? [];
    list.push(group);
    map.set(group.projectId, list);
  }
  for (const list of map.values()) {
    list.sort((a, b) => a.sort - b.sort);
  }
  return map;
});

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

function toggleProject(projectId: string): void {
  if (expandedProjects.value.has(projectId)) {
    expandedProjects.value.delete(projectId);
  } else {
    expandedProjects.value.add(projectId);
  }
  expandedProjects.value = new Set(expandedProjects.value);
}

function toggleGroup(groupId: string): void {
  if (expandedGroups.value.has(groupId)) {
    expandedGroups.value.delete(groupId);
  } else {
    expandedGroups.value.add(groupId);
  }
  expandedGroups.value = new Set(expandedGroups.value);
}

function ensureProjectExpanded(projectId: string): void {
  if (!expandedProjects.value.has(projectId)) {
    expandedProjects.value.add(projectId);
    expandedProjects.value = new Set(expandedProjects.value);
  }
}

function ensureGroupExpanded(groupId: string): void {
  if (!expandedGroups.value.has(groupId)) {
    expandedGroups.value.add(groupId);
    expandedGroups.value = new Set(expandedGroups.value);
  }
}

function selectProject(projectId: string): void {
  ensureProjectExpanded(projectId);
  emit('selectProject', projectId);
}

function selectGroup(groupId: string, projectId: string): void {
  ensureGroupExpanded(groupId);
  emit('selectGroup', groupId);
  ensureProjectExpanded(projectId);
}

function selectRequest(requestId: string, groupId: string, projectId: string): void {
  emit('selectRequest', requestId);
  ensureGroupExpanded(groupId);
  ensureProjectExpanded(projectId);
}

function handleDeleteProject(projectId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteProject', projectId);
}

function handleDeleteGroup(groupId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteGroup', groupId);
}

function handleDeleteRequest(requestId: string, event: Event): void {
  event.stopPropagation();
  emit('deleteRequest', requestId);
}

function handleCreateGroup(projectId: string, event: Event): void {
  event.stopPropagation();
  emit('createGroup', projectId);
}

function handleCreateRequest(groupId: string, groupName: string, event: Event): void {
  event.stopPropagation();
  emit('createRequest', { groupId, groupName });
}
</script>

<template>
  <div class="flex h-full flex-col gap-3 border-r bg-card p-3">
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
        aria-label="新建项目"
        @click="emit('createProject')"
      >
        <Plus class="size-4" />
      </button>
    </div>

    <div v-if="isLoadingProjects" class="flex items-center gap-2 text-xs text-muted-foreground">
      <Spinner />
      正在加载项目…
    </div>

    <div
      v-else-if="projectsSorted.length === 0"
      class="flex flex-1 items-center justify-center rounded-md border border-dashed bg-muted/20 p-6 text-center text-xs text-muted-foreground"
    >
      暂无项目，点击右上角 <Plus class="mx-1 inline size-3" /> 新建一个项目开始
    </div>

    <ScrollArea v-else class="min-h-0 flex-1">
      <ul class="flex flex-col gap-1 pr-2">
        <li v-for="project of projectsSorted" :key="project.id" class="flex flex-col gap-1">
          <div
            class="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-accent"
            :class="{ 'bg-accent': project.id === activeProjectId }"
          >
            <button
              type="button"
              class="flex flex-1 items-center gap-1 text-left text-sm font-medium"
              @click="toggleProject(project.id); selectProject(project.id)"
            >
              <ChevronDown v-if="expandedProjects.has(project.id)" class="size-4 text-muted-foreground" />
              <ChevronRight v-else class="size-4 text-muted-foreground" />
              <FolderOpen v-if="project.id === activeProjectId" class="size-4 text-primary" />
              <Folder v-else class="size-4 text-muted-foreground" />
              <span class="truncate">{{ project.name }}</span>
            </button>
            <button
              type="button"
              class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-foreground group-hover:opacity-100"
              :aria-label="`新建分组 ${project.name}`"
              @click="handleCreateGroup(project.id, $event)"
            >
              <Plus class="size-3.5" />
            </button>
            <button
              type="button"
              class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-destructive group-hover:opacity-100 disabled:opacity-50"
              :disabled="deletingProjectId === project.id"
              :aria-label="`删除项目 ${project.name}`"
              @click="handleDeleteProject(project.id, $event)"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>

          <ul v-if="expandedProjects.has(project.id)" class="flex flex-col gap-1 pl-4">
            <li v-if="isLoadingGroups" class="flex items-center gap-1 px-2 py-1 text-xs text-muted-foreground">
              <Spinner /> 加载分组…
            </li>

            <li
              v-for="group of groupsByProject.get(project.id) ?? []"
              v-else
              :key="group.id"
              class="flex flex-col gap-1"
            >
              <div
                class="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-accent"
                :class="{ 'bg-accent': group.id === activeGroupId }"
              >
                <button
                  type="button"
                  class="flex flex-1 items-center gap-1 text-left text-xs"
                  @click="toggleGroup(group.id); selectGroup(group.id, project.id)"
                >
                  <ChevronDown v-if="expandedGroups.has(group.id)" class="size-3.5 text-muted-foreground" />
                  <ChevronRight v-else class="size-3.5 text-muted-foreground" />
                  <Folder class="size-3.5 text-muted-foreground" />
                  <span class="truncate">{{ group.name }}</span>
                </button>
                <button
                  type="button"
                  class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-foreground group-hover:opacity-100"
                  :aria-label="`新建请求 ${group.name}`"
                  @click="handleCreateRequest(group.id, group.name, $event)"
                >
                  <Plus class="size-3" />
                </button>
                <button
                  type="button"
                  class="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-background hover:text-destructive group-hover:opacity-100 disabled:opacity-50"
                  :disabled="deletingGroupId === group.id"
                  :aria-label="`删除分组 ${group.name}`"
                  @click="handleDeleteGroup(group.id, $event)"
                >
                  <Trash2 class="size-3" />
                </button>
              </div>

              <ul v-if="expandedGroups.has(group.id)" class="flex flex-col gap-1 pl-5">
                <li
                  v-for="request of requestsByGroup.get(group.id) ?? []"
                  v-show="!searchKeyword || request.name.toLowerCase().includes(searchKeyword.toLowerCase())"
                  :key="request.id"
                  class="group flex items-center gap-1 rounded-md px-2 py-1 hover:bg-accent"
                  :class="{ 'bg-accent': request.id === activeRequestId }"
                >
                  <button
                    type="button"
                    class="flex flex-1 items-center gap-2 text-left text-xs"
                    @click="selectRequest(request.id, group.id, project.id)"
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
                  v-if="(requestsByGroup.get(group.id) ?? []).filter(request => !searchKeyword || request.name.toLowerCase().includes(searchKeyword.toLowerCase())).length === 0"
                  class="px-2 py-1 text-[11px] text-muted-foreground"
                >
                  暂无匹配请求
                </li>
              </ul>
            </li>

            <li
              v-if="(groupsByProject.get(project.id) ?? []).length === 0 && !isLoadingGroups"
              class="flex flex-col gap-2 rounded-md border border-dashed bg-muted/20 px-2 py-3 text-center text-[11px] text-muted-foreground"
            >
              项目中暂无分组
              <button
                type="button"
                class="mx-auto inline-flex items-center gap-1 rounded-md border bg-background px-2 py-1 text-[11px] text-foreground hover:bg-accent"
                @click="handleCreateGroup(project.id, $event)"
              >
                <Plus class="size-3" />
                新建分组
              </button>
            </li>
          </ul>
        </li>
      </ul>
    </ScrollArea>
  </div>
</template>

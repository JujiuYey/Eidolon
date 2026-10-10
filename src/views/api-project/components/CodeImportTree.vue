<script setup lang="ts">
import { ChevronDown, ChevronRight, Search } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';

import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { CodeImportModule, CodeImportResource } from '@/services/api-client/code-import/types';

interface Props {
  modules: CodeImportModule[];
  selectedKeys: ReadonlySet<string>;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:selectedKeys', value: Set<string>): void;
}>();

const keyword = ref('');
const expandedModules = ref<Set<string>>(new Set());
const expandedResources = ref<Set<string>>(new Set());

// Vue 不内置 indeterminate 绑定（DOM property 非 attribute），指令补上
const vIndeterminate = {
  mounted: (el: HTMLInputElement, binding: { value: boolean }) => {
    el.indeterminate = binding.value;
  },
  updated: (el: HTMLInputElement, binding: { value: boolean }) => {
    el.indeterminate = binding.value;
  },
};

const trimmedKeyword = computed(() => keyword.value.trim().toLowerCase());

interface VisibleModule {
  module: CodeImportModule;
  resources: { resource: CodeImportResource; matched: boolean }[];
  requestCount: number;
}

const visibleModules = computed<VisibleModule[]>(() => {
  const kw = trimmedKeyword.value;
  return props.modules
    .map(module => {
      const moduleMatched = !kw || module.name.toLowerCase().includes(kw);
      const resources = module.resources.map(resource => ({
        resource,
        matched:
          moduleMatched ||
          resource.name.toLowerCase().includes(kw) ||
          resource.fullName.toLowerCase().includes(kw) ||
          resource.operations.some(
            operation =>
              operation.label.toLowerCase().includes(kw) ||
              operation.action.toLowerCase().includes(kw) ||
              operation.externalKey.toLowerCase().includes(kw),
          ),
      }));
      const kept = kw && !moduleMatched ? resources.filter(item => item.matched) : resources;
      return {
        module,
        resources: kept,
        requestCount: kept.reduce((sum, item) => sum + item.resource.operations.length, 0),
      };
    })
    .filter(
      item =>
        item.resources.length > 0 && (!kw || item.requestCount > 0 || item.module.name.toLowerCase().includes(kw)),
    );
});

const selectedCount = computed(() => {
  let count = 0;
  for (const module of props.modules) {
    for (const resource of module.resources) {
      if (props.selectedKeys.has(resource.key)) {
        count += resource.operations.length;
      }
    }
  }
  return count;
});

function moduleState(module: CodeImportModule): 'all' | 'part' | 'none' {
  const total = module.resources.length;
  let selected = 0;
  for (const resource of module.resources) {
    if (props.selectedKeys.has(resource.key)) {
      selected += 1;
    }
  }
  if (selected === 0) {
    return 'none';
  }
  return selected === total ? 'all' : 'part';
}

function toggleModule(module: CodeImportModule): void {
  const next = new Set(props.selectedKeys);
  const state = moduleState(module);
  for (const resource of module.resources) {
    if (state === 'all') {
      next.delete(resource.key);
    } else {
      next.add(resource.key);
    }
  }
  emit('update:selectedKeys', next);
}

function toggleResource(resource: CodeImportResource): void {
  const next = new Set(props.selectedKeys);
  if (next.has(resource.key)) {
    next.delete(resource.key);
  } else {
    next.add(resource.key);
  }
  emit('update:selectedKeys', next);
}

function toggleIn(set: Set<string>, key: string): Set<string> {
  const next = new Set(set);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  return next;
}

// 搜索时自动展开，清空搜索后恢复手动折叠状态
watch(trimmedKeyword, keyword => {
  if (keyword) {
    expandedModules.value = new Set(props.modules.map(module => module.key));
    expandedResources.value = new Set();
  } else {
    expandedModules.value = new Set();
    expandedResources.value = new Set();
  }
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div class="flex items-center gap-3">
      <div class="relative w-64 max-w-full">
        <Search class="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input v-model="keyword" placeholder="搜索模块、资源或操作名" class="h-8 pl-8 text-sm" />
      </div>
      <p class="text-xs tabular-nums text-muted-foreground">已选 {{ selectedCount.toLocaleString() }} 个接口</p>
    </div>

    <ScrollArea class="min-h-0 flex-1 rounded-md border">
      <div
        v-if="visibleModules.length === 0"
        class="flex h-32 items-center justify-center text-sm text-muted-foreground"
      >
        没有匹配的接口
      </div>

      <div v-else class="py-1">
        <div v-for="entry of visibleModules" :key="entry.module.key">
          <div class="flex items-center gap-2 px-3 py-1.5">
            <button
              type="button"
              class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:text-foreground"
              :aria-label="expandedModules.has(entry.module.key) ? '折叠模块' : '展开模块'"
              @click="expandedModules = toggleIn(expandedModules, entry.module.key)"
            >
              <ChevronDown
                v-if="expandedModules.has(entry.module.key)"
                class="size-4 transition-transform duration-150 ease-out"
              />
              <ChevronRight v-else class="size-4 transition-transform duration-150 ease-out" />
            </button>
            <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5">
              <input
                type="checkbox"
                class="size-4 shrink-0 accent-primary"
                :checked="moduleState(entry.module) === 'all'"
                v-indeterminate="moduleState(entry.module) === 'part'"
                @change="toggleModule(entry.module)"
              />
              <span class="truncate text-sm font-medium text-foreground">{{ entry.module.name }}</span>
              <span class="ml-auto shrink-0 text-xs tabular-nums text-muted-foreground">
                {{ entry.resources.length }} 资源 · {{ entry.requestCount }} 接口
              </span>
            </label>
          </div>

          <div v-if="expandedModules.has(entry.module.key)">
            <div v-for="item of entry.resources" :key="item.resource.key">
              <div class="flex items-center gap-2 py-1 pl-10 pr-3">
                <button
                  type="button"
                  class="flex size-4 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:text-foreground"
                  :aria-label="expandedResources.has(item.resource.key) ? '折叠资源' : '展开资源'"
                  @click="expandedResources = toggleIn(expandedResources, item.resource.key)"
                >
                  <ChevronRight
                    class="size-3.5 transition-transform duration-150 ease-out"
                    :class="expandedResources.has(item.resource.key) && 'rotate-90'"
                  />
                </button>
                <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5">
                  <input
                    type="checkbox"
                    class="size-4 shrink-0 accent-primary"
                    :checked="selectedKeys.has(item.resource.key)"
                    @change="toggleResource(item.resource)"
                  />
                  <span class="truncate font-mono text-xs text-foreground">{{ item.resource.fullName }}</span>
                  <span class="ml-auto shrink-0 text-xs tabular-nums text-muted-foreground">
                    {{ item.resource.operations.length }}
                  </span>
                </label>
              </div>

              <div v-if="expandedResources.has(item.resource.key)" class="pb-1.5 pl-[4.25rem] pr-3">
                <div v-for="operation of item.resource.operations" :key="operation.externalKey" class="py-0.5">
                  <div class="flex items-center gap-2">
                    <span class="shrink-0 font-mono text-[10px] font-semibold text-muted-foreground/80">
                      {{ operation.method }}
                    </span>
                    <span class="truncate font-mono text-xs text-muted-foreground">{{ operation.label }}</span>
                    <Badge
                      v-if="operation.auth === 'none'"
                      variant="outline"
                      class="h-4 px-1.5 text-[10px] text-muted-foreground"
                    >
                      免鉴权
                    </Badge>
                    <span
                      v-if="operation.permission"
                      class="ml-auto truncate font-mono text-[11px] text-muted-foreground/70"
                      :title="operation.permission"
                    >
                      {{ operation.permission }}
                    </span>
                  </div>
                  <div
                    v-if="operation.paramsFields.length > 0 || operation.queryFields.length > 0"
                    class="flex flex-wrap gap-x-3 gap-y-0.5 pb-0.5 pl-3"
                  >
                    <span
                      v-for="field of operation.queryFields"
                      :key="`q:${field.name}`"
                      class="font-mono text-[11px] leading-4 text-muted-foreground/60"
                      :title="`query ${field.name}${field.required ? '（必填）' : ''}`"
                    >
                      ?{{ field.name }}
                    </span>
                    <span
                      v-for="field of operation.paramsFields"
                      :key="`b:${field.name}`"
                      class="font-mono text-[11px] leading-4 text-muted-foreground/70"
                      :title="`${field.name}：${field.type}${field.required ? '（必填）' : ''}`"
                    >
                      <span v-if="field.required" class="text-primary">*</span>{{ field.name
                      }}<template v-if="field.label">（{{ field.label }}）</template>
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  </div>
</template>

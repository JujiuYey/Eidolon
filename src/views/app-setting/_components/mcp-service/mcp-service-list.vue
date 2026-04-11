<script setup lang="ts">
import { computed } from 'vue';
import { PencilLine, Plus, Search, Settings2, Trash2, TriangleAlert } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import type { McpServiceCard } from './types';

const props = defineProps<{
  services: McpServiceCard[];
}>();

const emit = defineEmits<{
  (e: 'create'): void;
  (e: 'editService', serviceId: string): void;
  (e: 'updateEnabled', payload: { serviceId: string; enabled: boolean }): void;
}>();

const primaryEditableServiceId = computed(() => {
  return props.services.find(service => service.highlighted)?.id
    ?? props.services[0]?.id
    ?? null;
});

function openPrimaryEdit() {
  if (!primaryEditableServiceId.value) {
    return;
  }

  emit('editService', primaryEditableServiceId.value);
}

function updateEnabled(serviceId: string, value: boolean | string | number) {
  emit('updateEnabled', {
    serviceId,
    enabled: Boolean(value),
  });
}
</script>

<template>
  <ScrollArea class="h-full pr-3">
    <section class="space-y-5 pb-6">
      <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-2">
          <h1 class="text-2xl font-bold">
            MCP 服务
          </h1>
          <Button
            variant="ghost"
            size="icon-sm"
            class="size-8 rounded-full text-muted-foreground hover:text-foreground"
          >
            <Search class="size-4" />
            <span class="sr-only">搜索 MCP 服务</span>
          </Button>
        </div>

        <div class="flex flex-wrap items-center justify-end gap-2">
          <Button
            variant="ghost"
            size="icon-sm"
            class="size-8 rounded-full bg-destructive/8 text-destructive hover:bg-destructive/12 hover:text-destructive"
          >
            <TriangleAlert class="size-4" />
            <span class="sr-only">查看异常</span>
          </Button>
          <Button
            variant="outline"
            size="sm"
            class="rounded-full px-4"
            @click="openPrimaryEdit"
          >
            <PencilLine class="size-4" />
            <span>编辑</span>
          </Button>
          <Button
            variant="outline"
            size="sm"
            class="rounded-full px-4"
            @click="emit('create')"
          >
            <Plus class="size-4" />
            <span>添加</span>
          </Button>
        </div>
      </div>

      <article
        v-for="service of services"
        :key="service.id"
        class="rounded-[1.75rem] border px-5 py-5 shadow-sm transition-colors" :class="[
          service.highlighted
            ? 'border-destructive/45 bg-destructive/5'
            : 'border-border/70 bg-card',
        ]"
      >
        <div class="flex min-h-[132px] flex-col justify-between gap-8">
          <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div class="space-y-1">
              <h2 class="text-[1.35rem] font-semibold tracking-tight">
                {{ service.name }}
              </h2>
            </div>

            <div class="flex items-center gap-1.5 self-end sm:self-auto">
              <Switch
                :model-value="service.enabled"
                @update:model-value="updateEnabled(service.id, $event)"
              />
              <Button
                variant="ghost"
                size="icon-sm"
                class="size-8 rounded-full text-destructive/80 hover:bg-destructive/10 hover:text-destructive"
              >
                <Trash2 class="size-4" />
                <span class="sr-only">删除服务</span>
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                class="size-8 rounded-full text-muted-foreground hover:bg-accent hover:text-foreground"
                @click="emit('editService', service.id)"
              >
                <Settings2 class="size-4" />
                <span class="sr-only">查看配置</span>
              </Button>
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <Badge
              v-for="tag of service.tags"
              :key="`${service.id}-${tag.label}`"
              variant="outline"
              :class="tag.class"
            >
              {{ tag.label }}
            </Badge>
          </div>
        </div>
      </article>
    </section>
  </ScrollArea>
</template>

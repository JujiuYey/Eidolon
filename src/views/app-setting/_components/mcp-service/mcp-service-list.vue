<script setup lang="ts">
import { Loader2, Plus, Search, Settings2, Trash2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import type { McpService } from '@/types/mcp-service';

defineProps<{
  isLoading: boolean;
  services: McpService[];
}>();

const emit = defineEmits<{
  (e: 'create'): void;
  (e: 'editService', serviceId: string): void;
  (e: 'requestDelete', serviceId: string): void;
  (e: 'updateEnabled', payload: { serviceId: string; enabled: boolean }): void;
}>();

const primarySolidTagClass = 'border-transparent bg-primary text-primary-foreground hover:bg-primary';
const primaryOutlineTagClass = 'border-primary/20 bg-primary/10 text-primary hover:bg-primary/10';
const secondaryTagClass = 'border-transparent bg-secondary text-secondary-foreground hover:bg-secondary';
const mutedTagClass = 'border-border bg-muted text-muted-foreground hover:bg-muted';

function buildTags(service: McpService) {
  const tags = [
    {
      label: service.transport_type === 'stdio' ? 'STDIO' : 'HTTP',
      class: primaryOutlineTagClass,
    },
  ];

  if (service.discovery) {
    if (service.discovery.tools.length > 0) {
      tags.push({
        label: `${service.discovery.tools.length} 工具`,
        class: primarySolidTagClass,
      });
    }

    if (service.discovery.prompts.length > 0) {
      tags.push({
        label: `${service.discovery.prompts.length} 提示`,
        class: secondaryTagClass,
      });
    }

    if (service.discovery.resources.length + service.discovery.resource_templates.length > 0) {
      tags.push({
        label: `${service.discovery.resources.length + service.discovery.resource_templates.length} 资源`,
        class: secondaryTagClass,
      });
    }
  } else {
    tags.push({
      label: '未测试',
      class: mutedTagClass,
    });
  }

  return tags;
}

function discoverySummary(service: McpService) {
  if (!service.discovery) {
    return '保存后可测试连接，并发现工具、提示与资源。';
  }

  if (service.discovery.server_name) {
    return `${service.discovery.server_name}${service.discovery.server_version ? ` · ${service.discovery.server_version}` : ''}`;
  }

  return '连接成功，已获取服务能力。';
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
          <Button variant="ghost" size="icon-sm">
            <Search class="size-4" />
            <span class="sr-only">搜索 MCP 服务</span>
          </Button>
        </div>

        <div class="flex flex-wrap items-center justify-end gap-2">
          <Button size="sm" @click="emit('create')">
            <Plus class="size-4" />
            <span>新增服务</span>
          </Button>
        </div>
      </div>

      <div v-if="isLoading" class="flex items-center justify-center rounded-xl border border-dashed py-16">
        <Loader2 class="size-6 animate-spin text-muted-foreground" />
      </div>

      <div
        v-else-if="services.length === 0"
        class="rounded-xl border border-dashed bg-muted/10 px-6 py-12 text-center"
      >
        <h2 class="text-lg font-semibold text-foreground">
          还没有 MCP 服务
        </h2>
        <p class="mt-2 text-sm text-muted-foreground">
          先添加一个服务，然后在详情页里测试连接并发现工具、提示和资源。
        </p>
        <Button class="mt-5" @click="emit('create')">
          <Plus class="size-4" />
          新增第一个服务
        </Button>
      </div>

      <template v-else>
        <article
          v-for="service of services"
          :key="service.id"
          class="rounded-xl border border-border/70 bg-card px-5 py-5 shadow-sm transition-colors hover:border-primary/20"
        >
          <div class="flex min-h-[132px] flex-col justify-between gap-6">
            <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div class="min-w-0 space-y-2">
                <div class="flex flex-wrap items-center gap-2">
                  <h2 class="truncate text-[1.2rem] font-semibold tracking-tight">
                    {{ service.name }}
                  </h2>
                  <Badge v-if="!service.enabled" variant="outline" class="border-border bg-muted text-muted-foreground">
                    已禁用
                  </Badge>
                </div>

                <p v-if="service.description" class="text-sm leading-6 text-muted-foreground">
                  {{ service.description }}
                </p>
                <p v-else class="text-sm leading-6 text-muted-foreground">
                  {{ discoverySummary(service) }}
                </p>

                <p v-if="service.discovery?.tested_at" class="text-xs text-muted-foreground">
                  最近测试：{{ new Date(service.discovery.tested_at).toLocaleString('zh-CN') }}
                </p>
              </div>

              <div class="flex items-center gap-1.5 self-end sm:self-auto">
                <Switch
                  :model-value="service.enabled"
                  @update:model-value="updateEnabled(service.id, $event)"
                />
                <Button variant="ghost" size="icon-sm" @click="emit('requestDelete', service.id)">
                  <Trash2 class="size-4" />
                  <span class="sr-only">删除服务</span>
                </Button>
                <Button variant="ghost" size="icon-sm" @click="emit('editService', service.id)">
                  <Settings2 class="size-4" />
                  <span class="sr-only">查看配置</span>
                </Button>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <Badge
                v-for="tag of buildTags(service)"
                :key="`${service.id}-${tag.label}`"
                variant="outline"
                :class="tag.class"
              >
                {{ tag.label }}
              </Badge>
            </div>
          </div>
        </article>
      </template>
    </section>
  </ScrollArea>
</template>

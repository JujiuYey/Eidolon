<script setup lang="ts">
import { computed } from 'vue';
import { ArrowLeft, Bot, Cpu, PencilLine, Sparkles, Wrench } from 'lucide-vue-next';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { AgentProfile } from '@/types';
import type { McpService } from '@/types/mcp-service';

const props = defineProps<{
  profile: AgentProfile;
  mcpServices: McpService[];
}>();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'edit'): void;
}>();

const providerName = computed(() => {
  return PROVIDER_REGISTRY.find(provider => provider.provider_id === props.profile.providerId)?.name
    ?? props.profile.providerId;
});

const selectedServices = computed(() => {
  return props.mcpServices.filter(service => props.profile.enabledMcpServiceIds.includes(service.id));
});

const selectedTools = computed(() => {
  return selectedServices.value.flatMap(service =>
    (service.discovery?.tools ?? [])
      .filter(tool => tool.enabled)
      .map(tool => ({
        key: `${service.id}:${tool.name}`,
        label: tool.title || tool.name,
        serviceName: service.name,
      })),
  );
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b px-4 py-4">
      <div class="flex items-center justify-between gap-2">
        <Button variant="ghost" size="sm" class="px-0" @click="emit('back')">
          <ArrowLeft class="size-4" />
          返回 Agent 列表
        </Button>

        <Button variant="outline" size="sm" @click="emit('edit')">
          <PencilLine class="size-4" />
          编辑
        </Button>
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-6 p-4">
        <section class="rounded-xl border bg-card p-4 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Bot class="size-4 text-primary" />
            <span>{{ profile.name }}</span>
          </div>
          <p class="mt-3 text-sm leading-6 text-muted-foreground">
            {{ profile.description || '这个 Agent 还没有填写描述。' }}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Cpu class="size-4 text-primary" />
            <span>模型</span>
          </div>
          <div class="mt-3 space-y-2 text-sm text-muted-foreground">
            <p><span class="font-medium text-foreground">Provider：</span>{{ providerName }}</p>
            <p><span class="font-medium text-foreground">Model：</span>{{ profile.modelId }}</p>
            <p><span class="font-medium text-foreground">Temperature：</span>{{ profile.temperature || '未设置' }}</p>
            <p><span class="font-medium text-foreground">Max Tokens：</span>{{ profile.maxTokens || '未设置' }}</p>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-4 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Sparkles class="size-4 text-primary" />
            <span>提示词</span>
          </div>
          <p class="mt-3 whitespace-pre-wrap text-sm leading-6 text-muted-foreground">
            {{ profile.systemPrompt }}
          </p>
        </section>

        <section class="rounded-xl border bg-card p-4 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Wrench class="size-4 text-primary" />
            <span>MCP 服务</span>
          </div>
          <div class="mt-3 flex flex-wrap gap-2">
            <Badge
              v-for="service of selectedServices"
              :key="service.id"
              variant="outline"
              class="border-border bg-muted text-muted-foreground"
            >
              {{ service.name }}
            </Badge>
            <p v-if="selectedServices.length === 0" class="text-sm text-muted-foreground">
              暂未选择
            </p>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-4 shadow-sm">
          <div class="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Wrench class="size-4 text-primary" />
            <span>工具</span>
          </div>
          <div class="mt-3 flex flex-wrap gap-2">
            <Badge
              v-for="tool of selectedTools"
              :key="tool.key"
              variant="outline"
              class="border-border bg-muted text-muted-foreground"
            >
              {{ tool.label }}
            </Badge>
            <p v-if="selectedTools.length === 0" class="text-sm text-muted-foreground">
              所选 MCP 暂无可用工具
            </p>
          </div>
        </section>
      </div>
    </ScrollArea>
  </div>
</template>

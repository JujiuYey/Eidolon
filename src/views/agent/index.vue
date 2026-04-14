<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { Bot, PencilLine, Plus, Sparkles } from 'lucide-vue-next';
import { useRouter } from 'vue-router';
import { PROVIDER_REGISTRY } from '@/config/provider-registry';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { listAgentProfiles } from '@/services/agent-profile';

const router = useRouter();
const profiles = ref(listAgentProfiles());

const providerNameMap = new Map(
  PROVIDER_REGISTRY.map(provider => [provider.provider_id, provider.name]),
);

const profileCards = computed(() => {
  return profiles.value.map(profile => ({
    ...profile,
    providerName: providerNameMap.get(profile.providerId) ?? profile.providerId,
  }));
});

function loadProfiles() {
  profiles.value = listAgentProfiles();
}

function openCreatePage() {
  router.push('/agent/new');
}

function openAgent(profileId: string) {
  router.push(`/agent/${profileId}`);
}

function editAgent(profileId: string) {
  router.push(`/agent/${profileId}/edit`);
}

onMounted(() => {
  loadProfiles();
});

onActivated(() => {
  loadProfiles();
});
</script>

<template>
  <div class="mx-auto flex h-full max-w-6xl flex-col overflow-hidden p-6">
    <div class="mb-6 shrink-0 flex flex-col gap-4 border-b pb-5 lg:flex-row lg:items-end lg:justify-between">
      <div>
        <h1 class="flex items-center gap-2 text-2xl font-semibold tracking-tight text-foreground">
          <Bot class="size-6 text-primary" />
          Agent 工作台
        </h1>
        <p class="mt-2 text-sm text-muted-foreground">
          创建不同的 Agent 配置，分别绑定模型、提示词、MCP 服务和工具，然后进入各自独立的对话页。
        </p>
      </div>

      <Button @click="openCreatePage">
        <Plus class="size-4" />
        新建 Agent
      </Button>
    </div>

    <div
      v-if="profileCards.length === 0"
      class="flex flex-1 items-center justify-center"
    >
      <div class="w-full max-w-xl rounded-2xl border border-dashed bg-muted/10 px-8 py-12 text-center">
        <div class="mx-auto flex size-14 items-center justify-center rounded-full border bg-background">
          <Sparkles class="size-6 text-primary" />
        </div>
        <h2 class="mt-5 text-xl font-semibold text-foreground">
          还没有 Agent
        </h2>
        <p class="mt-2 text-sm leading-6 text-muted-foreground">
          先创建一个 Agent，配置模型、提示词和可用能力，之后就可以直接进入它的独立对话页。
        </p>
        <Button class="mt-6" @click="openCreatePage">
          <Plus class="size-4" />
          创建第一个 Agent
        </Button>
      </div>
    </div>

    <div v-else class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      <button
        v-for="profile of profileCards"
        :key="profile.id"
        type="button"
        class="rounded-2xl border bg-card p-5 text-left shadow-sm transition-colors hover:border-primary/30 hover:bg-primary/5"
        @click="openAgent(profile.id)"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <h2 class="truncate text-lg font-semibold tracking-tight text-foreground">
              {{ profile.name }}
            </h2>
            <p class="mt-2 line-clamp-2 min-h-[44px] text-sm leading-6 text-muted-foreground">
              {{ profile.description || '这个 Agent 还没有填写描述。' }}
            </p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <Badge variant="outline">
              {{ profile.modelId }}
            </Badge>
            <Button
              variant="ghost"
              size="icon-sm"
              @click.stop="editAgent(profile.id)"
            >
              <PencilLine class="size-4" />
              <span class="sr-only">编辑 Agent</span>
            </Button>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
            {{ profile.providerName }}
          </Badge>
          <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
            {{ profile.enabledMcpServiceIds.length }} MCP
          </Badge>
          <Badge variant="outline" class="border-border bg-muted text-muted-foreground">
            {{ profile.enabledToolKeys.length }} 工具
          </Badge>
        </div>

        <p class="mt-4 text-xs text-muted-foreground">
          更新于 {{ new Date(profile.updatedAt).toLocaleString('zh-CN') }}
        </p>
      </button>
    </div>
  </div>
</template>

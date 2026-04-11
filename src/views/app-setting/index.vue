<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Component } from 'vue';
import { Palette, HardDrive, Sparkles, Database, Server, Brain } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';

import AppConfig from './_components/app-config/index.vue';
import DataConfig from './_components/data-config/index.vue';
import McpService from './_components/mcp-service/index.vue';
import ModelConfig from './_components/model-config/index.vue';

interface SettingMenu {
  title: string;
  key: string;
  icon: Component;
  dividerAfter?: boolean;
}

const menus: SettingMenu[] = [
  {
    title: '模型服务',
    key: 'model-config',
    icon: Palette,
  },
  {
    title: '默认模型',
    key: 'default-model',
    icon: Sparkles,
    dividerAfter: true,
  },
  {
    title: '通用设置',
    key: 'app-config',
    icon: HardDrive,
  },
  {
    title: '数据设置',
    key: 'data',
    icon: Database,
    dividerAfter: true,
  },
  {
    title: 'MCP服务',
    key: 'mcp-service',
    icon: Server,
  },
  {
    title: 'Skills',
    key: 'skills',
    icon: Brain,
  },
];

const activeKey = ref('model-config');
const activeMenu = computed(() => menus.find(menu => menu.key === activeKey.value));

function handleClick(key: string) {
  activeKey.value = key;
}
</script>

<template>
  <div class="flex h-screen overflow-hidden">
    <!-- 侧边栏 -->
    <aside class="flex w-56 flex-col border-r bg-sidebar py-4">
      <div class="px-3 py-2">
        <h2 class="mb-2 px-2 text-sm font-semibold text-sidebar-foreground/70">
          设置
        </h2>
        <nav class="space-y-1">
          <template
            v-for="(item, index) of menus"
            :key="item.key"
          >
            <Button
              :variant="activeKey === item.key ? 'outline' : 'ghost'"
              class="w-full justify-start gap-3"
              @click="handleClick(item.key)"
            >
              <component :is="item.icon" class="h-4 w-4 shrink-0" />
              <span>{{ item.title }}</span>
            </Button>
            <Separator
              v-if="item.dividerAfter && index < menus.length - 1"
              class="my-2"
            />
          </template>
        </nav>
      </div>
    </aside>

    <!-- 主内容区 -->
    <main class="mx-auto flex max-w-7xl flex-1 flex-col overflow-hidden p-6">
      <div
        v-if="activeKey !== 'mcp-service'"
        class="mb-6"
      >
        <div class="flex items-center gap-2">
          <h1 class="text-2xl font-bold">
            {{ activeMenu?.title }}
          </h1>
        </div>
      </div>

      <div class="min-h-0 flex-1">
        <ModelConfig v-if="activeKey === 'model-config'" />
        <AppConfig v-if="activeKey === 'app-config'" />
        <DataConfig v-if="activeKey === 'data'" />
        <McpService v-if="activeKey === 'mcp-service'" />
      </div>
    </main>
  </div>
</template>

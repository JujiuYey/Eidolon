<script setup lang="ts">
import { ref } from 'vue';
import type { Component } from 'vue';
import { Palette, HardDrive, Sparkles, Database, Server, Brain, Mail, ListTodo, Settings } from 'lucide-vue-next';
import { Separator } from '@/components/ui/separator';

import AppConfig from './_components/app-config/index.vue';
import DataConfig from './_components/data-config/index.vue';
import DefaultModel from './_components/default-model/index.vue';
import McpService from './_components/mcp-service/index.vue';
import ProviderConfig from './_components/provider-config/index.vue';
import SkillsConfig from './_components/skills-config/index.vue';
import SmtpConfig from './_components/smtp-config/index.vue';
import ZentaoConfig from './_components/zentao-config/index.vue';

interface SettingMenu {
  title: string;
  key: string;
  icon: Component;
  dividerAfter?: boolean;
}

const menus: SettingMenu[] = [
  {
    title: '模型服务',
    key: 'provider-config',
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
    title: '邮件账户',
    key: 'smtp-config',
    icon: Mail,
  },
  {
    title: '禅道账户',
    key: 'zentao-config',
    icon: ListTodo,
  },
  {
    title: 'Skills',
    key: 'skills',
    icon: Brain,
  },
];

const activeKey = ref('provider-config');

function handleClick(key: string) {
  activeKey.value = key;
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <div class="flex min-h-0 flex-1 gap-4 px-6 py-4">
      <!-- 设置导航：与文档页文档树卡片同款 -->
      <aside class="flex w-60 shrink-0 flex-col overflow-hidden rounded-xl border bg-card">
        <div class="flex items-center gap-2 px-4 py-3">
          <Settings class="h-4 w-4 text-primary" />
          <h2 class="text-sm font-semibold">
            设置
          </h2>
        </div>
        <nav class="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 pb-2">
          <template
            v-for="(item, index) of menus"
            :key="item.key"
          >
            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left text-sm transition-colors"
              :class="activeKey === item.key
                ? 'bg-muted text-foreground'
                : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground'"
              @click="handleClick(item.key)"
            >
              <component
                :is="item.icon"
                class="h-4 w-4 shrink-0"
              />
              <span class="truncate">{{ item.title }}</span>
            </button>
            <Separator
              v-if="item.dividerAfter && index < menus.length - 1"
              class="my-2"
            />
          </template>
        </nav>
      </aside>

      <!-- 内容区：各分区自渲染一张等高卡片，与文档页编辑器卡片同款 -->
      <main class="flex min-w-0 flex-1">
        <ProviderConfig v-if="activeKey === 'provider-config'" />
        <DefaultModel v-if="activeKey === 'default-model'" />
        <AppConfig v-if="activeKey === 'app-config'" />
        <DataConfig v-if="activeKey === 'data'" />
        <McpService v-if="activeKey === 'mcp-service'" />
        <SmtpConfig v-if="activeKey === 'smtp-config'" />
        <ZentaoConfig v-if="activeKey === 'zentao-config'" />
        <SkillsConfig v-if="activeKey === 'skills'" />
      </main>
    </div>
  </div>
</template>

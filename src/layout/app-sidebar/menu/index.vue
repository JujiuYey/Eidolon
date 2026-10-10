<script lang="ts" setup>
import { Bot, FileText, Mail, NotebookPen, Network, ListTodo, Server } from 'lucide-vue-next';
import type { Component } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import { useApiClientStore } from '@/stores/api-client';

import { isMenuItemActive } from './utils';
import type { MenuItemMatchTarget } from './utils';

interface MenuItem extends MenuItemMatchTarget {
  title: string;
  icon: Component;
  /** 图标模式收起时隐藏：动态系统项收起后只剩相同图标，认不出谁是谁 */
  hideCollapsed?: boolean;
}

interface MenuGroup {
  key: string;
  label?: string;
  items: MenuItem[];
}

const router = useRouter();
const route = useRoute();
const store = useApiClientStore();

const toolsGroup: MenuGroup = {
  key: 'tools',
  label: '工具',
  items: [
    { title: '邮件', key: 'mail', icon: Mail, path: '/mail' },
    { title: '周报', key: 'weekly-report', icon: NotebookPen, path: '/weekly-report' },
    { title: '禅道', key: 'zentao', icon: ListTodo, path: '/zentao' },
  ],
};
const knowledgeGroup: MenuGroup = {
  key: 'knowledge',
  label: '知识库',
  items: [{ title: '文档', key: 'docs', icon: FileText, path: '/docs' }],
};
const agentGroup: MenuGroup = {
  key: 'agent',
  label: '智能体',
  items: [{ title: '产品经理', key: 'pm', icon: Bot, path: '/pm' }],
};

const apiMenuGroup = computed<MenuGroup>(() => ({
  key: 'api-client',
  label: 'API 管理',
  items: [
    ...store.sortedProjects.map(project => ({
      title: project.name,
      key: `api-project-${project.id}`,
      icon: Server,
      path: `/api-client/projects/${project.id}`,
      projectId: project.id,
      hideCollapsed: true,
    })),
    { title: '管理项目', key: 'api-client', icon: Network, path: '/api-client', exact: true },
  ],
}));

const menuGroups = computed<MenuGroup[]>(() => [toolsGroup, apiMenuGroup.value, knowledgeGroup, agentGroup]);

onMounted(() => {
  store.loadProjects().catch(() => {
    toast.error('加载 API 项目列表失败');
  });
});

function handleClick(menu: MenuItem) {
  router.push(menu.path);
}
const getMenuButtonClass = computed(() => (item: MenuItem) => ({
  'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground': isMenuItemActive(item, route),
}));
</script>

<template>
  <SidebarContent>
    <SidebarGroup v-for="group of menuGroups" :key="group.key">
      <SidebarGroupLabel v-if="group.label">{{ group.label }}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          <SidebarMenuItem
            v-for="item of group.items"
            :key="item.key"
            :class="item.hideCollapsed ? 'group-data-[collapsible=icon]:hidden' : undefined"
            @click="handleClick(item)"
          >
            <SidebarMenuButton
              as-child
              :class="getMenuButtonClass(item)"
              :tooltip="item.hideCollapsed ? undefined : item.title"
            >
              <span>
                <component :is="item.icon" />
                <span>{{ item.title }}</span>
              </span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  </SidebarContent>
</template>

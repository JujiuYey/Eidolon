<script lang="ts" setup>
import {
  Bot,
  FolderClosed,
  Mail,
  NotebookPen,
  Network,
  ListTodo,
  Server,
  Settings2,
  SquareCheckBig,
} from 'lucide-vue-next';
import type { Component } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import { useApiClientStore } from '@/stores/api-client';
import { useDocsStore } from '@/stores/docs';
import { docsRootLabels } from '@/views/docs/utils/roots';

import { isMenuItemActive } from './utils';
import type { MenuItemMatchTarget } from './utils';

interface MenuItem extends MenuItemMatchTarget {
  title: string;
  icon: Component;
  /** 图标模式收起时隐藏：动态项收起后只剩相同图标，认不出谁是谁 */
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
const docsStore = useDocsStore();

const toolsGroup: MenuGroup = {
  key: 'tools',
  label: '工具',
  items: [
    { title: '邮件', key: 'mail', icon: Mail, path: '/mail' },
    { title: '周报', key: 'weekly-report', icon: NotebookPen, path: '/weekly-report' },
    { title: '禅道', key: 'zentao', icon: ListTodo, path: '/zentao' },
    { title: '任务', key: 'tasks', icon: SquareCheckBig, path: '/tasks' },
  ],
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
    { title: '管理项目', key: 'api-client', icon: Network, path: '/api-client', exact: true },
    ...store.sortedProjects.map(project => ({
      title: project.name,
      key: `api-project-${project.id}`,
      icon: Server,
      path: `/api-client/projects/${project.id}`,
      routeMatch: { name: 'api-client-project', paramKey: 'id', paramValue: project.id },
      hideCollapsed: true,
    })),
  ],
}));

const knowledgeGroup = computed<MenuGroup>(() => {
  const labels = docsRootLabels(docsStore.roots);
  return {
    key: 'knowledge',
    label: '知识库',
    items: [
      { title: '管理目录', key: 'docs', icon: Settings2, path: '/docs', exact: true },
      ...docsStore.roots.map(root => ({
        title: labels.get(root.id) ?? root.path,
        key: `docs-root-${root.id}`,
        icon: FolderClosed,
        path: `/docs/r/${root.id}`,
        routeMatch: { name: 'docs-root', paramKey: 'rootId', paramValue: root.id },
        hideCollapsed: true,
      })),
    ],
  };
});

const menuGroups = computed<MenuGroup[]>(() => [toolsGroup, apiMenuGroup.value, knowledgeGroup.value, agentGroup]);

onMounted(() => {
  store.loadProjects().catch(() => {
    toast.error('加载 API 项目列表失败');
  });
  docsStore.initialize().catch(() => {
    toast.error('加载文档库列表失败');
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

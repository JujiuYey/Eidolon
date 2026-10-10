<script lang="ts" setup>
import { Bot, FileText, Mail, NotebookPen, Network, ListTodo } from 'lucide-vue-next';
import type { Component } from 'vue';
import { useRoute, useRouter } from 'vue-router';

interface MenuItem {
  title: string;
  key: string;
  icon: Component;
  path: string;
}

interface MenuGroup {
  key: string;
  label?: string;
  items: MenuItem[];
}

const router = useRouter();
const route = useRoute();

const menuGroups: MenuGroup[] = [
  {
    key: 'tools',
    label: '工具',
    items: [
      { title: '邮件', key: 'mail', icon: Mail, path: '/mail' },
      { title: '周报', key: 'weekly-report', icon: NotebookPen, path: '/weekly-report' },
      { title: '禅道', key: 'zentao', icon: ListTodo, path: '/zentao' },
    ],
  },
  {
    key: 'api-client',
    items: [{ title: 'API 管理', key: 'api-client', icon: Network, path: '/api-client' }],
  },
  {
    key: 'knowledge',
    label: '知识库',
    items: [{ title: '文档', key: 'docs', icon: FileText, path: '/docs' }],
  },
  {
    key: 'agent',
    label: '智能体',
    items: [{ title: '产品经理', key: 'pm', icon: Bot, path: '/pm' }],
  },
];

const currentKey = computed(() => {
  return route.path.split('/')[1] || 'index';
});

function handleClick(menu: MenuItem) {
  router.push(menu.path);
}
const getMenuButtonClass = computed(() => (key: string) => ({
  'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground': key === currentKey.value,
}));
</script>

<template>
  <SidebarContent>
    <SidebarGroup v-for="group of menuGroups" :key="group.key">
      <SidebarGroupLabel v-if="group.label">{{ group.label }}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          <SidebarMenuItem v-for="item of group.items" :key="item.key" @click="handleClick(item)">
            <SidebarMenuButton as-child :class="getMenuButtonClass(item.key)">
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

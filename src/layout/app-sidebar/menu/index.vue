<script lang="ts" setup>
import type { Component } from 'vue';
import { Bot, Mail, NotebookPen, Network, ListTodo } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';

interface Menu {
  title: string;
  key: string;
  icon: Component;
  path: string;
}

const router = useRouter();
const route = useRoute();

const menus: Menu[] = [
  {
    title: '邮件',
    key: 'mail',
    icon: Mail,
    path: '/mail',
  },
  {
    title: '周报',
    key: 'weekly-report',
    icon: NotebookPen,
    path: '/weekly-report',
  },
  {
    title: '禅道',
    key: 'zentao',
    icon: ListTodo,
    path: '/zentao',
  },
  {
    title: '接口请求',
    key: 'api-client',
    icon: Network,
    path: '/api-client',
  },
  {
    title: '产品经理',
    key: 'pm',
    icon: Bot,
    path: '/pm',
  },
];

const currentKey = computed(() => {
  return route.path.split('/')[1] || 'index';
});

function handleClick(menu: Menu) {
  router.push(menu.path);
}
const getMenuButtonClass = computed(() => (key: string) => ({
  'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground': key === currentKey.value,
}));
</script>

<template>
  <SidebarContent>
    <SidebarGroup>
      <SidebarGroupLabel>应用</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          <SidebarMenuItem
            v-for="item of menus"
            :key="item.key"
            @click="handleClick(item)"
          >
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

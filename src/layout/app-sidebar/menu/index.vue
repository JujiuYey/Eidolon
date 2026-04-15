<script lang="ts" setup>
import type { Component } from 'vue';
import { MessageCircle, Bot, Sparkles } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import AppSidebarRecentConversations from '../recent-conversations/index.vue';

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
    title: '开始',
    key: 'index',
    icon: MessageCircle,
    path: '/index',
  },
  {
    title: '智能体管理',
    key: 'agent',
    icon: Bot,
    path: '/agent',
  },
  {
    title: 'CRUD 生成',
    key: 'codegen',
    icon: Sparkles,
    path: '/codegen',
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

    <AppSidebarRecentConversations />
  </SidebarContent>
</template>

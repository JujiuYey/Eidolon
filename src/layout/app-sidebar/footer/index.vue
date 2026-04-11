<script setup lang="ts">
import { PanelLeftOpen, PanelLeftClose, Settings } from 'lucide-vue-next';
import { useSidebar } from '@/components/ui/sidebar';
import { useRouter, useRoute } from 'vue-router';

const { state, toggleSidebar } = useSidebar();
const router = useRouter();
const route = useRoute();

const currentKey = computed(() => route.path.split('/')[1] || 'index');

function handleClick() {
  router.push('/app-setting');
}
const getMenuButtonClass = computed(() => {
  return {
    'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground': currentKey.value === 'app-setting',
  };
});
</script>

<template>
  <SidebarFooter>
    <SidebarMenu>
      <SidebarMenuItem @click="handleClick">
        <SidebarMenuButton as-child :class="getMenuButtonClass">
          <span>
            <Settings />
            <span>应用设置</span>
          </span>
        </SidebarMenuButton>
      </SidebarMenuItem>

      <SidebarMenuItem @click="toggleSidebar">
        <SidebarMenuButton as-child>
          <span v-if="state === 'collapsed'">
            <PanelLeftOpen class="size-4 cursor-pointer" />
            <span>展开</span>
          </span>
          <span v-else>
            <PanelLeftClose class="size-4 cursor-pointer" />
            <span>收起</span>
          </span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  </SidebarFooter>
</template>

<script setup lang="ts">
import { Settings } from 'lucide-vue-next';
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

const currentKey = computed(() => route.path.split('/')[1] || 'index');

function handleClick() {
  router.push('/app-setting');
}
const getMenuButtonClass = computed(() => {
  return {
    'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground':
      currentKey.value === 'app-setting',
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
    </SidebarMenu>
  </SidebarFooter>
</template>

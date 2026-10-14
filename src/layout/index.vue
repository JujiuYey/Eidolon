<script lang="ts" setup>
import SidebarProvider from '@/components/ui/sidebar/SidebarProvider.vue';

import AppSidebarFooter from './app-sidebar/footer/index.vue';
import Logo from './app-sidebar/logo/index.vue';
import AppSidebarMenu from './app-sidebar/menu/index.vue';
</script>

<template>
  <SidebarProvider :default-open="true">
    <Sidebar collapsible="icon">
      <Logo />
      <AppSidebarMenu />
      <AppSidebarFooter />
      <SidebarRail />
    </Sidebar>
    <div class="w-[0px] flex-1 h-screen overflow-hidden">
      <RouterView />
    </div>
  </SidebarProvider>
</template>

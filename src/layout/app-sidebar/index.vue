<script setup lang="ts">
import AppSidebarHeader from './header/index.vue';
import AppSidebarFooter from './footer/index.vue';
import AppSidebarMenu from './menu/index.vue';
</script>

<template>
  <Sidebar collapsible="icon">
    <AppSidebarHeader />
    <AppSidebarMenu />
    <AppSidebarFooter />
    <SidebarRail />
  </Sidebar>
</template>

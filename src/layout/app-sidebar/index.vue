<script setup lang="ts">
import AppSidebarFooter from './footer/index.vue';
import Logo from './logo/index.vue';
import AppSidebarMenu from './menu/index.vue';
</script>

<template>
  <Sidebar collapsible="icon">
    <Logo />
    <AppSidebarMenu />
    <AppSidebarFooter />
    <SidebarRail />
  </Sidebar>
</template>

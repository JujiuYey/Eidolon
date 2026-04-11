<script lang="ts" setup>
</script>

<template>
  <SidebarHeader>
    <SidebarMenuButton
      size="lg"
      class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
    >
      <div class="flex items-center justify-center rounded-lg aspect-square size-8">
        <img src="/src/assets/logo.svg" alt="logo" class="size-6" />
      </div>
      <div class="grid flex-1 text-sm leading-tight text-left">
        <span class="font-semibold truncate">Eidolon</span>
      </div>
    </SidebarMenuButton>
  </SidebarHeader>
</template>

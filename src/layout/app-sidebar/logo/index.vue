<script lang="ts" setup>
import { PanelLeftClose } from 'lucide-vue-next';

import { useSidebar } from '@/components/ui/sidebar';

const { toggleSidebar } = useSidebar();
</script>

<template>
  <SidebarHeader>
    <SidebarMenuButton
      as="div"
      size="lg"
      class="flex justify-between bg-primary text-white hover:bg-primary/90 hover:text-white"
    >
      <div class="flex items-center gap-2">
        <div class="flex items-center justify-center rounded-lg aspect-square size-8" @click="toggleSidebar">
          <img src="/src/assets/logo.svg" alt="logo" class="size-10" />
        </div>
        <div class="grid flex-1 leading-tight text-left">
          <span class="font-semibold truncate">Eidolon</span>
        </div>
      </div>

      <Button
        variant="ghost"
        size="icon"
        class="size-7 shrink-0 text-white hover:bg-white/20 hover:text-white"
        title="收起"
        @click="toggleSidebar"
      >
        <PanelLeftClose class="size-4" />
      </Button>
    </SidebarMenuButton>
  </SidebarHeader>
</template>

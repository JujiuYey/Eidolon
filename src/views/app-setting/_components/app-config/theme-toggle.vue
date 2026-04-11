<script setup lang="ts">
import type { Theme } from '@/types';
import { Computer, Moon, Sun } from 'lucide-vue-next';
import { useTheme } from '@/composables/use-theme';

const { theme, setTheme } = useTheme();

function handleThemeChange(value: string | number) {
  if (value === 'light' || value === 'dark' || value === 'system') {
    setTheme(value as Theme);
  }
}
</script>

<template>
  <Tabs :model-value="theme" @update:model-value="handleThemeChange">
    <TabsList>
      <TabsTrigger value="light">
        <Sun class="h-[1.2rem] w-[1.2rem]" />
        浅色
      </TabsTrigger>
      <TabsTrigger value="dark">
        <Moon class="h-[1.2rem] w-[1.2rem]" />
        深色
      </TabsTrigger>
      <TabsTrigger value="system">
        <Computer class="h-[1.2rem] w-[1.2rem]" />
        系统
      </TabsTrigger>
    </TabsList>
  </Tabs>
</template>

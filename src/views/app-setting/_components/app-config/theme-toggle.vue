<script setup lang="ts">
import { Computer, Moon, Sun } from 'lucide-vue-next';
import { useTheme } from '@/composables/use-theme';

const { theme, setTheme } = useTheme();
</script>

<template>
  <Tabs :default-value="theme">
    <TabsList>
      <TabsTrigger value="light" @click="setTheme('light')">
        <Sun class="h-[1.2rem] w-[1.2rem]" />
        浅色
      </TabsTrigger>
      <TabsTrigger value="dark" @click="setTheme('dark')">
        <Moon class="h-[1.2rem] w-[1.2rem]" />
        深色
      </TabsTrigger>
      <TabsTrigger value="system" @click="setTheme('system')">
        <Computer class="h-[1.2rem] w-[1.2rem]" />
        系统
      </TabsTrigger>
    </TabsList>
  </tabs>
</template>

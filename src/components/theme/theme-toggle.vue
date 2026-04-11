<script setup lang="ts">
import { Moon, Sun } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useTheme } from '@/composables/use-theme';

const { setTheme } = useTheme();
</script>

<template>
  <DropdownMenu>
    <DropdownMenuTrigger as-child>
      <Button variant="outline" size="icon">
        <Sun class="h-[1.2rem] w-[1.2rem] rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
        <Moon class="absolute h-[1.2rem] w-[1.2rem] rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
        <span class="sr-only">切换主题</span>
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      <DropdownMenuItem @click="setTheme('light')">
        浅色
      </DropdownMenuItem>
      <DropdownMenuItem @click="setTheme('dark')">
        深色
      </DropdownMenuItem>
      <DropdownMenuItem @click="setTheme('system')">
        系统
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
</template>

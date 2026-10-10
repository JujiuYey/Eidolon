<script setup lang="ts">
import { HardDrive } from 'lucide-vue-next';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import ThemeColorToggle from './theme-color-toggle.vue';
import ThemeToggle from './theme-toggle.vue';
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card">
    <div class="flex items-center gap-2 px-4 py-3">
      <HardDrive class="h-4 w-4 text-primary" />
      <h2 class="text-sm font-semibold">
        通用设置
      </h2>
    </div>
    <Separator />

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-4 px-5 py-4">
        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>主题设置</Label>
            <p class="text-sm text-muted-foreground">
              选择应用主题外观
            </p>
          </div>
          <ThemeToggle />
        </div>

        <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div class="space-y-0.5">
            <Label>主题颜色</Label>
            <p class="text-sm text-muted-foreground">
              为按钮、高亮和聚焦态选择一套主色
            </p>
          </div>
          <ThemeColorToggle />
        </div>
      </div>
    </ScrollArea>
  </div>
</template>

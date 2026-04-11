<script setup lang="ts">
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import ThemeColorToggle from './theme-color-toggle.vue';
import ThemeToggle from './theme-toggle.vue';
</script>

<template>
  <Card class="h-full">
    <CardHeader>
      <CardTitle class="text-lg">
        应用设置
      </CardTitle>
      <CardDescription>
        个性化应用体验
      </CardDescription>
    </CardHeader>

    <ScrollArea class="flex-1 min-h-0">
      <CardContent class="space-y-4 ">
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

        <Separator />
      </CardContent>
    </ScrollArea>
  </Card>
</template>

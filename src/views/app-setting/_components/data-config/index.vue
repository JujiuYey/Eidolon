<script setup lang="ts">
import { ref } from 'vue';
import type { Component } from 'vue';
import {
  ArchiveRestore,
  FileOutput,
  FolderOpen,
  FolderUp,
  Wifi,
} from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';

interface RowAction {
  label: string;
  icon?: Component;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  class?: string;
}

interface DirectoryRow {
  title: string;
  hint?: string;
  value?: string;
  valueIcon?: Component;
  action: RowAction;
}

const compactBackup = ref(false);

const appDataPath = '/Users/jujiuyey/Library/Application Support/Eidolon';
const appLogPath = `${appDataPath}/logs`;

const dangerOutlineClass = 'border-destructive/60 text-destructive hover:bg-destructive/10 hover:text-destructive dark:border-destructive/50';

const backupActions: RowAction[] = [
  {
    label: '备份',
    icon: ArchiveRestore,
    variant: 'outline',
  },
  {
    label: '恢复',
    icon: FolderUp,
    variant: 'outline',
  },
];

const exportActions: Array<{ title: string; action: RowAction }> = [
  {
    title: '局域网传输',
    action: {
      label: '开始传输',
      icon: Wifi,
      variant: 'outline',
    },
  },
  {
    title: '导出为文件',
    action: {
      label: '导出到文件',
      icon: FileOutput,
      variant: 'outline',
    },
  },
];

const directoryRows: DirectoryRow[] = [
  {
    title: '应用数据',
    value: appDataPath,
    valueIcon: FolderOpen,
    action: {
      label: '打开目录',
      variant: 'outline',
    },
  },
  {
    title: '应用日志',
    value: appLogPath,
    action: {
      label: '打开日志',
      variant: 'outline',
    },
  },
  {
    title: '知识库文件',
    action: {
      label: '删除文件',
      variant: 'outline',
    },
  },
  {
    title: '清除缓存',
    hint: '(0.91MB)',
    action: {
      label: '清除缓存',
      variant: 'outline',
    },
  },
  {
    title: '重置数据',
    action: {
      label: '重置数据',
      variant: 'outline',
      class: dangerOutlineClass,
    },
  },
];
</script>

<template>
  <ScrollArea class="h-full pr-3">
    <div class="space-y-5 pb-6">
      <Card class="gap-0 overflow-hidden rounded-[1.75rem] border-border/70 py-0 shadow-sm">
        <CardHeader class="px-5 pb-2 pt-5">
          <CardTitle class="text-lg">
            数据设置
          </CardTitle>
        </CardHeader>

        <CardContent class="px-5 pb-2 pt-0">
          <div class="flex flex-col gap-4 py-4 sm:flex-row sm:items-center sm:justify-between">
            <div class="space-y-1">
              <h3 class="text-base font-medium">
                数据备份与恢复
              </h3>
            </div>

            <div class="flex flex-wrap justify-end gap-2">
              <Button
                v-for="action of backupActions"
                :key="action.label"
                :variant="action.variant"
                size="sm"
              >
                <component :is="action.icon" class="size-4" />
                <span>{{ action.label }}</span>
              </Button>
            </div>
          </div>

          <Separator />

          <div class="flex flex-col gap-4 py-4 sm:flex-row sm:items-center sm:justify-between">
            <div class="space-y-1">
              <h3 class="text-base font-medium">
                精简备份
              </h3>
              <p class="max-w-3xl text-sm leading-6 text-muted-foreground">
                备份时跳过备份图片、知识库等数据文件，仅备份聊天记录和设置，减少空间占用，加快备份速度
              </p>
            </div>

            <Switch v-model="compactBackup" />
          </div>
        </CardContent>
      </Card>

      <Card class="gap-0 overflow-hidden rounded-[1.75rem] border-border/70 py-0 shadow-sm">
        <CardHeader class="px-5 pb-2 pt-5">
          <CardTitle class="text-lg">
            导出至手机
          </CardTitle>
        </CardHeader>

        <CardContent class="px-5 pb-2 pt-0">
          <template
            v-for="(item, index) of exportActions"
            :key="item.title"
          >
            <div class="flex flex-col gap-4 py-4 sm:flex-row sm:items-center sm:justify-between">
              <h3 class="text-base font-medium">
                {{ item.title }}
              </h3>

              <Button
                :variant="item.action.variant"
                size="sm"
              >
                <component :is="item.action.icon" class="size-4" />
                <span>{{ item.action.label }}</span>
              </Button>
            </div>

            <Separator v-if="index < exportActions.length - 1" />
          </template>
        </CardContent>
      </Card>

      <Card class="gap-0 overflow-hidden rounded-[1.75rem] border-border/70 py-0 shadow-sm">
        <CardHeader class="px-5 pb-2 pt-5">
          <CardTitle class="text-lg">
            数据目录
          </CardTitle>
        </CardHeader>

        <CardContent class="px-5 pb-2 pt-0">
          <template
            v-for="(item, index) of directoryRows"
            :key="item.title"
          >
            <div class="grid gap-3 py-4 sm:grid-cols-[minmax(0,180px)_1fr_auto] sm:items-center sm:gap-6">
              <div class="min-w-0">
                <div class="flex items-baseline gap-1.5">
                  <h3 class="text-base font-medium">
                    {{ item.title }}
                  </h3>
                  <span
                    v-if="item.hint"
                    class="text-sm text-muted-foreground"
                  >
                    {{ item.hint }}
                  </span>
                </div>
              </div>

              <div
                v-if="item.value"
                class="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground"
              >
                <span class="truncate">{{ item.value }}</span>
                <component
                  :is="item.valueIcon"
                  v-if="item.valueIcon"
                  class="size-3.5 shrink-0"
                />
              </div>
              <div v-else class="hidden sm:block" />

              <Button
                :variant="item.action.variant"
                :class="item.action.class"
                size="sm"
                class="justify-self-start sm:justify-self-end"
              >
                {{ item.action.label }}
              </Button>
            </div>

            <Separator v-if="index < directoryRows.length - 1" />
          </template>
        </CardContent>
      </Card>
    </div>
  </ScrollArea>
</template>

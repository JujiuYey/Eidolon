<script setup lang="ts">
import { FilePlus2, LayoutTemplate } from 'lucide-vue-next';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { MailTemplate } from '@/types/mail';

defineProps<{
  templates: MailTemplate[];
}>();

const emit = defineEmits<{
  (e: 'manage'): void;
}>();

const selectedTemplateId = defineModel<string>('selectedTemplateId', {
  required: true,
});
</script>

<template>
  <ScrollArea class="flex h-[calc(100vh-56px)]">
    <div class="flex flex-col gap-2 p-4 pt-0">
      <button
        type="button"
        :class="cn(
          'flex flex-col items-start gap-1 rounded-lg border p-3 text-left text-sm transition-all hover:bg-accent',
          selectedTemplateId === '' && 'bg-muted border-primary/40',
        )"
        @click="selectedTemplateId = ''"
      >
        <div class="flex items-center gap-2 font-semibold">
          <FilePlus2 class="size-4 text-muted-foreground" />
          空白邮件
        </div>
        <div class="text-xs text-muted-foreground">
          不使用模板，从零开始写
        </div>
      </button>

      <button
        v-for="template of templates"
        :key="template.id"
        type="button"
        :class="cn(
          'flex flex-col items-start gap-1 rounded-lg border p-3 text-left text-sm transition-all hover:bg-accent',
          selectedTemplateId === template.id && 'bg-muted border-primary/40',
        )"
        @click="selectedTemplateId = template.id"
      >
        <div class="w-full truncate font-semibold">
          {{ template.name }}
        </div>
        <div class="line-clamp-2 text-xs text-muted-foreground">
          {{ template.subject || '（无主题）' }}
        </div>
      </button>

      <div
        v-if="templates.length === 0"
        class="rounded-lg border border-dashed p-4 text-center text-xs text-muted-foreground"
      >
        还没有模板，点击下方按钮创建
      </div>

      <Button
        type="button"
        variant="outline"
        class="mt-2"
        @click="emit('manage')"
      >
        <LayoutTemplate class="mr-2 h-4 w-4" />
        管理模板
      </Button>
    </div>
  </ScrollArea>
</template>

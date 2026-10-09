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
  <ScrollArea class="min-h-0 flex-1">
    <div class="flex flex-col gap-2 px-4 pb-4">
      <button
        type="button"
        :class="cn(
          'flex flex-col items-start gap-1 rounded-lg border p-3 text-left text-sm outline-none transition-colors duration-150 hover:border-ring/40 hover:bg-muted/40 focus-visible:ring-[3px] focus-visible:ring-ring/50',
          selectedTemplateId === '' && 'border-primary/40 bg-muted',
        )"
        @click="selectedTemplateId = ''"
      >
        <div class="flex items-center gap-2 font-medium">
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
          'flex flex-col items-start gap-1 rounded-lg border p-3 text-left text-sm outline-none transition-colors duration-150 hover:border-ring/40 hover:bg-muted/40 focus-visible:ring-[3px] focus-visible:ring-ring/50',
          selectedTemplateId === template.id && 'border-primary/40 bg-muted',
        )"
        @click="selectedTemplateId = template.id"
      >
        <div class="w-full truncate font-medium">
          {{ template.name }}
        </div>
        <div class="line-clamp-2 w-full text-xs text-muted-foreground">
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

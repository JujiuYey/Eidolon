<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { RotateCcw, Trash2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { useMailStore } from '@/stores/mail';
import type { SentEmail } from '@/types/mail';
import { getErrorMessage } from '@/utils/helpers';

defineProps<{
  mail: SentEmail | null;
}>();

const emit = defineEmits<{
  (e: 'resend', mail: SentEmail): void;
}>();

const store = useMailStore();
const deleteConfirmOpen = ref(false);

const sentAtText = computed(() => {
  const mail = store.selectedSentEmail;
  if (!mail?.sent_at) {
    return '-';
  }
  return new Date(mail.sent_at).toLocaleString('zh-CN', { hour12: false });
});

async function handleDelete() {
  const mail = store.selectedSentEmail;
  if (!mail) {
    return;
  }
  try {
    await store.removeSentEmail(mail.id);
    deleteConfirmOpen.value = false;
    toast.success('已删除发送记录');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

function handleResend() {
  const mail = store.selectedSentEmail;
  if (!mail) {
    return;
  }
  emit('resend', mail);
}
</script>

<template>
  <div
    v-if="mail"
    class="flex h-full flex-col"
  >
    <div class="flex items-start justify-between gap-4 p-4">
      <div class="flex-1 space-y-1">
        <h2 class="text-xl font-bold break-all">
          {{ mail.subject || '（无主题）' }}
        </h2>
        <p class="text-sm text-muted-foreground">
          {{ sentAtText }}
        </p>
      </div>
      <Badge
        :variant="mail.status === 'sent' ? 'secondary' : 'destructive'"
        class="shrink-0"
      >
        {{ mail.status === 'sent' ? '已发送' : '发送失败' }}
      </Badge>
    </div>
    <Separator />
    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-4 p-4 text-sm">
        <div class="space-y-1 rounded-lg border bg-muted/30 p-3">
          <p class="text-xs text-muted-foreground">
            发件人：<span class="text-foreground">{{ mail.account_email || '-' }}</span>
          </p>
          <p class="text-xs text-muted-foreground">
            收件人：<span class="text-foreground">{{ mail.to_addresses || '-' }}</span>
          </p>
          <p
            v-if="mail.cc_addresses"
            class="text-xs text-muted-foreground"
          >
            抄送：<span class="text-foreground">{{ mail.cc_addresses }}</span>
          </p>
        </div>

        <p
          v-if="mail.error_message"
          class="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive"
        >
          {{ mail.error_message }}
        </p>

        <pre class="font-sans break-words whitespace-pre-wrap text-sm leading-6">{{ mail.body }}</pre>
      </div>
    </ScrollArea>
    <Separator />
    <div class="flex items-center justify-end gap-3 p-4">
      <Button
        type="button"
        variant="outline"
        class="h-9"
        @click="handleResend"
      >
        <RotateCcw class="mr-2 h-4 w-4" />
        重新发送
      </Button>
      <Button
        type="button"
        variant="ghost"
        class="h-9 text-destructive hover:text-destructive"
        @click="deleteConfirmOpen = true"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除记录
      </Button>
    </div>
  </div>

  <div
    v-else
    class="flex h-full items-center justify-center p-8"
  >
    <div class="text-center text-sm text-muted-foreground">
      <p>未选择发送记录</p>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除这条发送记录吗？"
    description="删除后不可恢复，邮件本身不会受影响。"
    type="destructive"
    @confirm="handleDelete"
  />
</template>

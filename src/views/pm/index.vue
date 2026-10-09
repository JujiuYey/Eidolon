<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import { Separator } from '@/components/ui/separator';
import { listDefaultModelSettings } from '@/services';
import { usePmStore } from '@/stores/pm';
import ChatInput from './_components/chat-input.vue';
import ConversationList from './_components/conversation-list.vue';
import MessageList from './_components/message-list.vue';

const router = useRouter();
const store = usePmStore();

const chatInputRef = ref<InstanceType<typeof ChatInput> | null>(null);
const modelLabel = ref('');
const modelConfigured = ref(false);

async function loadModelLabel() {
  try {
    const settings = await listDefaultModelSettings();
    const setting = settings.find(item => item.key === 'pm_chat');
    if (setting) {
      modelLabel.value = `${setting.provider_id} / ${setting.model_id}`;
      modelConfigured.value = true;
    } else {
      modelLabel.value = '未配置默认模型';
    }
  } catch {
    modelLabel.value = '';
  }
}

async function handleSend(content: string) {
  if (!modelConfigured.value) {
    toast.error('请先配置产品经理模型', {
      description: '在「应用设置 → 默认模型 → 产品经理」中选择一个模型。',
      action: {
        label: '去设置',
        onClick: () => router.push('/app-setting'),
      },
    });
    return;
  }

  const ok = await store.send(content);
  if (ok) {
    chatInputRef.value?.clear();
  } else {
    chatInputRef.value?.focus();
  }
}

onMounted(() => {
  void store.loadConversations();
  void loadModelLabel();
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <div class="px-6 pt-6">
      <SagPageHeader title="产品经理">
        <template #meta>
          {{ modelLabel }}
        </template>
      </SagPageHeader>
      <Separator class="mt-4 shrink-0" />
    </div>

    <div class="flex min-h-0 flex-1 gap-4 px-6 py-4">
      <aside class="w-72 shrink-0">
        <ConversationList
          :active-id="store.activeConversationId"
          :conversations="store.conversations"
          :is-loading="store.conversationsLoading"
          @create="store.createConversation().catch(error => toast.error(String(error)))"
          @remove="store.removeConversation($event).catch(error => toast.error(String(error)))"
          @rename="(id, title) => store.renameConversation(id, title).catch(error => toast.error(String(error)))"
          @select="store.selectConversation($event)"
        />
      </aside>

      <div class="flex min-w-0 flex-1 flex-col rounded-xl border bg-card">
        <MessageList
          :has-conversation="!!store.activeConversation"
          :is-loading="store.messagesLoading"
          :messages="store.messages"
          :sending="store.sending"
          @send="handleSend"
        />
        <ChatInput
          ref="chatInputRef"
          :error="store.sendError"
          :sending="store.sending"
          @send="handleSend"
        />
      </div>
    </div>
  </div>
</template>

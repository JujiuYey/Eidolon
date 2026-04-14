<script setup lang="ts">
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import AgentProfileEditor from './components/AgentProfileEditor.vue';
import { upsertAgentProfile } from '@/services/agent-profile-storage';
import type { AgentProfileInput } from '@/types';

const router = useRouter();

function handleCancel() {
  router.push('/agent');
}

async function handleSave(value: AgentProfileInput) {
  try {
    const profile = await upsertAgentProfile(value);
    toast.success('Agent 已创建');
    router.push(`/agent/${profile.id}`);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '创建 Agent 失败');
  }
}
</script>

<template>
  <div class="h-full overflow-hidden p-6">
    <AgentProfileEditor mode="create" @cancel="handleCancel" @save="handleSave" />
  </div>
</template>

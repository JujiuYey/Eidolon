<script setup lang="ts">
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import AgentProfileEditor from './components/AgentProfileEditor.vue';
import { upsertAgentProfile } from '@/services/agent-profile';
import type { AgentProfileInput } from '@/types';

const router = useRouter();

function handleCancel() {
  router.push('/agent');
}

function handleSave(value: AgentProfileInput) {
  const profile = upsertAgentProfile(value);
  toast.success('Agent 已创建');
  router.push(`/agent/${profile.id}`);
}
</script>

<template>
  <div class="h-full overflow-hidden p-6">
    <AgentProfileEditor mode="create" @cancel="handleCancel" @save="handleSave" />
  </div>
</template>

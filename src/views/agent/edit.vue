<script setup lang="ts">
import { computed } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import AgentProfileEditor from './components/AgentProfileEditor.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  getAgentProfile,
  upsertAgentProfile,
} from '@/services/agent-profile';
import type { AgentProfileInput } from '@/types';

const route = useRoute();
const router = useRouter();

const profileId = computed(() => String(route.params.id ?? ''));
const profile = computed(() => getAgentProfile(profileId.value));

function handleCancel() {
  if (profile.value) {
    router.push(`/agent/${profile.value.id}`);
    return;
  }

  router.push('/agent');
}

function handleSave(value: AgentProfileInput) {
  const updated = upsertAgentProfile({
    ...value,
    id: profile.value?.id ?? value.id,
  });
  toast.success('Agent 已更新');
  router.push(`/agent/${updated.id}`);
}
</script>

<template>
  <div v-if="!profile" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
    <Alert class="max-w-xl">
      <AlertCircle class="size-4" />
      <AlertTitle>没有找到这个 Agent</AlertTitle>
      <AlertDescription>
        这个 Agent 可能已经不存在了，所以现在无法编辑。
      </AlertDescription>
      <div class="mt-4">
        <Button @click="router.push('/agent')">
          返回 Agent 列表
        </Button>
      </div>
    </Alert>
  </div>

  <div v-else class="h-full overflow-hidden p-6">
    <AgentProfileEditor
      mode="edit"
      :initial-profile="profile"
      @cancel="handleCancel"
      @save="handleSave"
    />
  </div>
</template>

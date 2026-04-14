<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { useRoute, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';
import AgentProfileEditor from './components/AgentProfileEditor.vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  getAgentProfile,
  upsertAgentProfile,
} from '@/services/agent-profile-storage';
import type {
  AgentProfile,
  AgentProfileInput,
} from '@/types';

const route = useRoute();
const router = useRouter();

const profileId = computed(() => String(route.params.id ?? ''));
const profile = ref<AgentProfile | null>(null);
const isLoading = ref(true);

async function loadProfile() {
  isLoading.value = true;

  try {
    profile.value = await getAgentProfile(profileId.value);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '加载 Agent 失败');
    profile.value = null;
  } finally {
    isLoading.value = false;
  }
}

function handleCancel() {
  if (profile.value) {
    router.push(`/agent/${profile.value.id}`);
    return;
  }

  router.push('/agent');
}

async function handleSave(value: AgentProfileInput) {
  try {
    const updated = await upsertAgentProfile({
      ...value,
      id: profile.value?.id ?? value.id,
    });
    toast.success('Agent 已更新');
    router.push(`/agent/${updated.id}`);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : '更新 Agent 失败');
  }
}

watch(profileId, () => {
  void loadProfile();
}, { immediate: true });
</script>

<template>
  <div v-if="isLoading" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
    <div class="text-sm text-muted-foreground">
      正在加载 Agent...
    </div>
  </div>

  <div v-else-if="!profile" class="mx-auto flex h-full max-w-3xl items-center justify-center p-6">
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

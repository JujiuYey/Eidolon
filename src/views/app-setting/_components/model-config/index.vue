<script setup lang="ts">
import { listModelConfigs } from '@/services/model_config';
import type { ModelConfig } from '@/services/model_config';
import ModelConfigList from './model-config-list.vue';
import ModelConfigPanel from './model-config-panel.vue';

const modelConfigList = shallowRef<ModelConfig[]>([]);
const selectedConfigId = ref<string | null>(null);

async function fetchList() {
  const nextList = await listModelConfigs();
  modelConfigList.value = nextList;

  if (nextList.length === 0) {
    selectedConfigId.value = null;
    return;
  }

  if (!selectedConfigId.value || !nextList.some(config => config.id === selectedConfigId.value)) {
    selectedConfigId.value = nextList[0]!.id;
  }
}

onMounted(() => {
  fetchList();
});

function handleRefresh() {
  fetchList();
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ModelConfigList
        :model-config-list="modelConfigList"
        :selected-config-id="selectedConfigId"
      />
      <ModelConfigPanel
        :model-config-list="modelConfigList"
        :selected-config-id="selectedConfigId"
        @update:selected-config-id="selectedConfigId = $event"
        @refresh="handleRefresh"
      />
    </div>
  </div>
</template>

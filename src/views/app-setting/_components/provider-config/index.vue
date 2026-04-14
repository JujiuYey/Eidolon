<script setup lang="ts">
import { onMounted } from 'vue';
import { useProviderStore } from '@/stores/provider';
import ProviderList from './_components/provider-list.vue';
import ProviderConfigPanel from './_components/provider-config-panel.vue';

const store = useProviderStore();

onMounted(() => {
  void store.loadConfigs();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ProviderList />
      <ProviderConfigPanel v-if="store.selectedView" />
    </div>
  </div>
</template>

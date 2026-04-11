<script setup lang="ts">
import ProviderList from './_components/provider-list.vue';
import ProviderConfigPanel from './_components/provider-config-panel.vue';

const selectedProviderId = ref('minimax');
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ProviderList v-model:selected-provider-id="selectedProviderId" />
      <ProviderConfigPanel :selected-provider-id="selectedProviderId" />
    </div>
  </div>
</template>

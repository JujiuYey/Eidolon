<script setup lang="ts">
import { computed, ref } from 'vue';
import McpServiceForm from './mcp-service-form.vue';
import McpServiceList from './mcp-service-list.vue';
import type { McpServiceCard, McpServiceFormMode } from './types';

const blueSolidTagClass = 'border-transparent bg-sky-500 text-white hover:bg-sky-500';
const blueOutlineTagClass = 'border-sky-400/70 bg-sky-50 text-sky-600 hover:bg-sky-50 dark:border-sky-500/60 dark:bg-sky-500/10 dark:text-sky-300 dark:hover:bg-sky-500/10';
const greenOutlineTagClass = 'border-lime-400/70 bg-lime-50 text-lime-700 hover:bg-lime-50 dark:border-lime-500/60 dark:bg-lime-500/10 dark:text-lime-300 dark:hover:bg-lime-500/10';

const services = ref<McpServiceCard[]>([
  {
    id: 'stdio-host',
    name: 'MCP 服务器',
    enabled: false,
    highlighted: true,
    tags: [
      {
        label: 'STDIO',
        class: blueOutlineTagClass,
      },
    ],
  },
  {
    id: 'cherry-fetch',
    name: '@cherry/fetch',
    enabled: true,
    tags: [
      {
        label: '0.1.0',
        class: blueSolidTagClass,
      },
      {
        label: '内置',
        class: blueOutlineTagClass,
      },
      {
        label: 'CherryAI',
        class: greenOutlineTagClass,
      },
    ],
  },
  {
    id: 'vortex',
    name: 'vortex',
    enabled: true,
    tags: [
      {
        label: '0.1.0',
        class: blueSolidTagClass,
      },
      {
        label: 'STDIO',
        class: blueOutlineTagClass,
      },
    ],
  },
]);

type McpServiceViewState
  = | { type: 'list' }
    | { type: 'form'; mode: McpServiceFormMode; serviceId?: string };

const viewState = ref<McpServiceViewState>({ type: 'list' });

const activeService = computed(() => {
  const currentView = viewState.value;

  if (currentView.type !== 'form' || !currentView.serviceId) {
    return null;
  }

  return services.value.find(service => service.id === currentView.serviceId) ?? null;
});

function openCreateForm() {
  viewState.value = {
    type: 'form',
    mode: 'create',
  };
}

function openEditForm(serviceId: string) {
  viewState.value = {
    type: 'form',
    mode: 'edit',
    serviceId,
  };
}

function backToList() {
  viewState.value = { type: 'list' };
}

function updateServiceEnabled(payload: { serviceId: string; enabled: boolean }) {
  const target = services.value.find(service => service.id === payload.serviceId);

  if (!target) {
    return;
  }

  target.enabled = payload.enabled;
}
</script>

<template>
  <McpServiceList
    v-if="viewState.type === 'list'"
    :services="services"
    @create="openCreateForm"
    @edit-service="openEditForm"
    @update-enabled="updateServiceEnabled"
  />
  <McpServiceForm
    v-else
    :mode="viewState.mode"
    :service="activeService"
    @back="backToList"
  />
</template>

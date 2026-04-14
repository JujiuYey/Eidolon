<script setup lang="ts">
import { computed, ref } from 'vue';
import McpServiceForm from './mcp-service-form.vue';
import McpServiceList from './mcp-service-list.vue';
import type { McpServiceCard, McpServiceFormMode } from './types';

const primarySolidTagClass = 'border-transparent bg-primary text-primary-foreground hover:bg-primary';
const primaryOutlineTagClass = 'border-primary/20 bg-primary/10 text-primary hover:bg-primary/10';
const secondaryTagClass = 'border-transparent bg-secondary text-secondary-foreground hover:bg-secondary';

const services = ref<McpServiceCard[]>([
  {
    id: 'stdio-host',
    name: 'MCP 服务器',
    enabled: false,
    highlighted: true,
    tags: [
      {
        label: 'STDIO',
        class: primaryOutlineTagClass,
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
        class: primarySolidTagClass,
      },
      {
        label: '内置',
        class: primaryOutlineTagClass,
      },
      {
        label: 'CherryAI',
        class: secondaryTagClass,
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
        class: primarySolidTagClass,
      },
      {
        label: 'STDIO',
        class: primaryOutlineTagClass,
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

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import {
  deleteMcpService,
  listMcpServices,
  upsertMcpService,
} from '@/services/mcp_service';
import type { McpService } from '@/types/mcp-service';
import { getErrorMessage } from '@/utils/helpers';
import McpServiceForm from './mcp-service-form.vue';
import McpServiceList from './mcp-service-list.vue';
import type { McpServiceFormMode } from './types';

type McpServiceViewState
  = | { type: 'list' }
    | { type: 'form'; mode: McpServiceFormMode; serviceId?: string };

const viewState = ref<McpServiceViewState>({ type: 'list' });
const services = ref<McpService[]>([]);
const isLoading = ref(false);
const deleteConfirmOpen = ref(false);
const pendingDeleteService = ref<McpService | null>(null);

const activeService = computed(() => {
  const currentView = viewState.value;

  if (currentView.type !== 'form' || !currentView.serviceId) {
    return null;
  }

  return services.value.find(service => service.id === currentView.serviceId) ?? null;
});

async function loadServices() {
  isLoading.value = true;

  try {
    services.value = await listMcpServices();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载 MCP 服务失败'));
  } finally {
    isLoading.value = false;
  }
}

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

async function updateServiceEnabled(payload: { serviceId: string; enabled: boolean }) {
  const target = services.value.find(service => service.id === payload.serviceId);

  if (!target) {
    return;
  }

  const previousEnabled = target.enabled;
  target.enabled = payload.enabled;

  try {
    await upsertMcpService({
      ...target,
      enabled: payload.enabled,
    });
    toast.success(payload.enabled ? '已启用 MCP 服务' : '已禁用 MCP 服务');
  } catch (error) {
    target.enabled = previousEnabled;
    toast.error(getErrorMessage(error, '更新 MCP 服务状态失败'));
  }
}

function requestDelete(serviceId: string) {
  pendingDeleteService.value = services.value.find(service => service.id === serviceId) ?? null;
  deleteConfirmOpen.value = Boolean(pendingDeleteService.value);
}

async function confirmDelete() {
  if (!pendingDeleteService.value) {
    return;
  }

  try {
    await deleteMcpService(pendingDeleteService.value.id);
    deleteConfirmOpen.value = false;
    pendingDeleteService.value = null;
    await loadServices();
    backToList();
    toast.success('MCP 服务已删除');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除 MCP 服务失败'));
  }
}

async function handleSaved(serviceId: string) {
  await loadServices();
  openEditForm(serviceId);
}

async function handleRemoved(serviceId: string) {
  if (activeService.value?.id === serviceId) {
    backToList();
  }
  await loadServices();
}

onMounted(() => {
  void loadServices();
});
</script>

<template>
  <McpServiceList
    v-if="viewState.type === 'list'"
    :is-loading="isLoading"
    :services="services"
    @create="openCreateForm"
    @edit-service="openEditForm"
    @request-delete="requestDelete"
    @update-enabled="updateServiceEnabled"
  />
  <McpServiceForm
    v-else
    :mode="viewState.mode"
    :service="activeService"
    @removed="handleRemoved"
    @saved="handleSaved"
    @back="backToList"
  />

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除这个 MCP 服务吗？"
    :description="pendingDeleteService ? `删除后会移除 ${pendingDeleteService.name} 的配置和工具偏好` : ''"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>

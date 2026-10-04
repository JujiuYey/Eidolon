<script lang="ts" setup>
import { onMounted } from 'vue';
import { toast } from 'vue-sonner';
import { useMailStore } from '@/stores/mail';
import { getErrorMessage } from '@/utils/helpers';
import Mail from './components/Mail.vue';

const store = useMailStore();

onMounted(async () => {
  try {
    await store.loadAll();
  } catch (error) {
    toast.error(getErrorMessage(error, '加载邮件数据失败'));
  }
});
</script>

<template>
  <Mail />
</template>

<script setup lang="ts">
import type { ZentaoAccount } from '@/types/zentao';
import { computed, onMounted, ref } from 'vue';
import { ListTodo, Plus, RefreshCw, Settings2 } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import ConfirmDialog from '@/components/ConfirmDialog.vue';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useConfirm } from '@/composables/use-confirm';
import { useZentaoStore } from '@/stores/zentao';
import { getErrorMessage } from '@/utils/helpers';
import AccountDialog from './_components/account-dialog.vue';
import BugList from './_components/bug-list.vue';
import TaskList from './_components/task-list.vue';
import { formatTimestamp, sortBugs, sortTasks } from './utils/display';

const store = useZentaoStore();
const confirm = useConfirm();

const dialogOpen = ref(false);
const editingAccount = ref<ZentaoAccount | null>(null);

const taskCount = computed(() => store.myWork?.tasks.length ?? 0);
const bugCount = computed(() => store.myWork?.bugs.length ?? 0);
const hasAccount = computed(() => store.accounts.length > 0);

const sortedTasks = computed(() => sortTasks(store.myWork?.tasks ?? []));
const sortedBugs = computed(() => sortBugs(store.myWork?.bugs ?? []));

onMounted(async () => {
  try {
    await store.loadAccounts();
    if (store.activeAccountId) {
      await refreshWork(false);
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '加载禅道账户失败'));
  }
});

async function refreshWork(notify = true) {
  await store.refreshWork();
  if (store.workError) {
    toast.error(store.workError);
    return;
  }
  for (const warning of store.myWork?.warnings ?? []) {
    toast.warning(warning);
  }
  if (notify) {
    toast.success(
      `已刷新：${store.myWork?.tasks.length ?? 0} 个任务、${store.myWork?.bugs.length ?? 0} 个 Bug`,
    );
  }
}

function handleAccountChange(value: unknown) {
  if (typeof value !== 'string') {
    return;
  }
  store.selectAccount(value);
  void refreshWork(false);
}

function openCreateDialog() {
  editingAccount.value = null;
  dialogOpen.value = true;
}

function openEditDialog() {
  editingAccount.value = store.activeAccount;
  dialogOpen.value = true;
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <header class="flex items-center gap-3 px-6 pt-6">
      <h1 class="text-lg font-semibold">
        禅道任务
      </h1>
      <p
        v-if="store.myWork"
        class="text-xs text-muted-foreground"
      >
        更新于 {{ formatTimestamp(store.myWork.fetched_at) }}
      </p>

      <div class="ml-auto flex items-center gap-2">
        <template v-if="hasAccount">
          <Select
            :model-value="store.activeAccountId"
            @update:model-value="handleAccountChange"
          >
            <SelectTrigger class="w-52">
              <SelectValue placeholder="选择账户" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="account of store.accounts"
                :key="account.id"
                :value="account.id"
              >
                {{ account.name }}（{{ account.account }}）
              </SelectItem>
            </SelectContent>
          </Select>

          <Button
            variant="outline"
            size="sm"
            :disabled="store.workLoading || !store.activeAccountId"
            @click="openEditDialog"
          >
            <Settings2 class="mr-1 h-4 w-4" />
            编辑账户
          </Button>
        </template>

        <Button
          variant="outline"
          size="sm"
          @click="openCreateDialog"
        >
          <Plus class="mr-1 h-4 w-4" />
          新建账户
        </Button>

        <Button
          v-if="hasAccount"
          size="sm"
          :disabled="store.workLoading || !store.activeAccountId"
          @click="refreshWork()"
        >
          <RefreshCw
            class="mr-1 h-4 w-4"
            :class="store.workLoading && 'animate-spin'"
          />
          {{ store.workLoading ? '拉取中…' : '刷新' }}
        </Button>
      </div>
    </header>

    <Separator class="mt-4" />

    <!-- 未配置账户的空状态 -->
    <div
      v-if="!hasAccount && !store.accountsLoading"
      class="flex flex-1 flex-col items-center justify-center gap-3 py-24 text-center"
    >
      <div class="flex h-12 w-12 items-center justify-center rounded-full bg-muted">
        <ListTodo class="h-6 w-6 text-muted-foreground" />
      </div>
      <div class="space-y-1">
        <p class="text-sm font-medium">
          还没有配置禅道账户
        </p>
        <p class="max-w-sm text-xs text-muted-foreground">
          填写禅道站点地址与登录账号，即可在这里直接查看名下的任务与 Bug，无需打开禅道网页。
        </p>
      </div>
      <Button
        size="sm"
        @click="openCreateDialog"
      >
        <Plus class="mr-1 h-4 w-4" />
        配置禅道账户
      </Button>
    </div>

    <!-- 主内容 -->
    <Tabs
      v-else
      default-value="tasks"
      class="flex flex-1 flex-col px-6 pb-6"
    >
      <TabsList class="w-fit">
        <TabsTrigger value="tasks">
          我的任务（{{ taskCount }}）
        </TabsTrigger>
        <TabsTrigger value="bugs">
          我的 Bug（{{ bugCount }}）
        </TabsTrigger>
      </TabsList>

      <TabsContent
        value="tasks"
        class="mt-4"
      >
        <div
          v-if="store.workLoading && !store.myWork"
          class="py-16 text-center text-sm text-muted-foreground"
        >
          正在从禅道拉取任务…
        </div>
        <div
          v-else-if="taskCount === 0"
          class="py-16 text-center text-sm text-muted-foreground"
        >
          没有进行中的任务，休息一下吧。
        </div>
        <TaskList
          v-else
          :tasks="sortedTasks"
        />
      </TabsContent>

      <TabsContent
        value="bugs"
        class="mt-4"
      >
        <div
          v-if="store.workLoading && !store.myWork"
          class="py-16 text-center text-sm text-muted-foreground"
        >
          正在从禅道拉取 Bug…
        </div>
        <div
          v-else-if="bugCount === 0"
          class="py-16 text-center text-sm text-muted-foreground"
        >
          没有未关闭的 Bug。
        </div>
        <BugList
          v-else
          :bugs="sortedBugs"
        />
      </TabsContent>
    </Tabs>

    <!-- 删除账户走模块级 useConfirm 单例，此处负责渲染 -->
    <ConfirmDialog
      :open="confirm.state.open"
      :title="confirm.state.title"
      :description="confirm.state.description"
      :confirm-label="confirm.state.confirmLabel"
      :cancel-label="confirm.state.cancelLabel"
      :destructive="confirm.state.destructive"
      @update:open="confirm.onOpenChange"
      @confirm="confirm.onConfirm"
      @cancel="confirm.onCancel"
    />

    <AccountDialog
      v-model:open="dialogOpen"
      :account="editingAccount"
    />
  </div>
</template>

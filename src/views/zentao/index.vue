<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import {
  Bug as BugIcon,
  CircleDotDashed,
  CloudOff,
  ListTodo,
  RefreshCw,
  Settings,
} from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useZentaoStore } from '@/stores/zentao';
import { getErrorMessage } from '@/utils/helpers';
import BugList from './_components/bug-list.vue';
import TaskList from './_components/task-list.vue';
import { formatTimestamp, sortBugs, sortTasks } from './utils/display';

const store = useZentaoStore();
const router = useRouter();

const tasks = computed(() => store.myWork?.tasks ?? []);
const bugs = computed(() => store.myWork?.bugs ?? []);
const doingCount = computed(() => tasks.value.filter(task => task.status === 'doing').length);
const hasAccount = computed(() => store.accounts.length > 0);

const sortedTasks = computed(() => sortTasks(tasks.value));
const sortedBugs = computed(() => sortBugs(bugs.value));

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
    toast.success(`已刷新：${tasks.value.length} 个任务、${bugs.value.length} 个 Bug`);
  }
}

function handleAccountChange(value: unknown) {
  if (typeof value !== 'string') {
    return;
  }
  store.selectAccount(value);
  void refreshWork(false);
}

/** 账户管理统一在应用设置，功能页只负责选择 */
function goSettings() {
  router.push('/app-setting');
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <SagPageHeader
      class="px-6 pt-6"
      title="禅道"
    >
      <template #meta>
        <span v-if="store.myWork">更新于 {{ formatTimestamp(store.myWork.fetched_at) }}</span>
      </template>
      <template #actions>
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
        </template>

        <Button
          v-else
          variant="outline"
          size="sm"
          @click="goSettings"
        >
          <Settings class="mr-1 h-4 w-4" />
          去设置添加账户
        </Button>
      </template>
    </SagPageHeader>

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
          在应用设置中添加禅道站点与登录账号后，即可在这里查看名下的任务与 Bug。
        </p>
      </div>
      <Button
        size="sm"
        @click="goSettings"
      >
        <Settings class="mr-1 h-4 w-4" />
        前往应用设置
      </Button>
    </div>

    <template v-else>
      <!-- 统计卡 -->
      <div
        v-if="store.myWork"
        class="grid grid-cols-3 gap-3 px-6 pt-4"
      >
        <Card class="py-4">
          <CardContent class="flex items-center gap-3 px-4">
            <div class="flex size-9 items-center justify-center rounded-md bg-primary/10">
              <ListTodo class="size-4.5 text-primary" />
            </div>
            <div>
              <p class="text-xl leading-none font-semibold tabular-nums">
                {{ tasks.length }}
              </p>
              <p class="mt-1 text-xs text-muted-foreground">
                进行中任务
              </p>
            </div>
          </CardContent>
        </Card>
        <Card class="py-4">
          <CardContent class="flex items-center gap-3 px-4">
            <div class="flex size-9 items-center justify-center rounded-md bg-primary/10">
              <CircleDotDashed class="size-4.5 text-primary" />
            </div>
            <div>
              <p class="text-xl leading-none font-semibold tabular-nums">
                {{ doingCount }}
              </p>
              <p class="mt-1 text-xs text-muted-foreground">
                正在开发
              </p>
            </div>
          </CardContent>
        </Card>
        <Card class="py-4">
          <CardContent class="flex items-center gap-3 px-4">
            <div class="flex size-9 items-center justify-center rounded-md bg-destructive/10">
              <BugIcon class="size-4.5 text-destructive" />
            </div>
            <div>
              <p class="text-xl leading-none font-semibold tabular-nums">
                {{ bugs.length }}
              </p>
              <p class="mt-1 text-xs text-muted-foreground">
                未关闭 Bug
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      <Separator class="mt-5" />

      <!-- 列表切换 + 内容 -->
      <div class="flex-1 px-6 py-4">
        <!-- 拉取失败：明确呈现错误与重试入口，而非伪装成空数据 -->
        <div
          v-if="store.workError && !store.myWork"
          class="flex flex-col items-center justify-center gap-3 rounded-lg border border-destructive/30 bg-destructive/5 py-20 text-center"
        >
          <CloudOff class="size-8 text-destructive/70" />
          <div class="space-y-1">
            <p class="text-sm font-medium">
              禅道数据拉取失败
            </p>
            <p class="max-w-md text-xs leading-5 text-muted-foreground">
              {{ store.workError }}
            </p>
          </div>
          <Button
            size="sm"
            variant="outline"
            :disabled="store.workLoading"
            @click="refreshWork(false)"
          >
            <RefreshCw
              class="mr-1 h-4 w-4"
              :class="store.workLoading && 'animate-spin'"
            />
            重试
          </Button>
        </div>

        <div
          v-else-if="store.workLoading && !store.myWork"
          class="flex flex-col items-center justify-center gap-2 py-20 text-muted-foreground"
        >
          <RefreshCw class="size-5 animate-spin" />
          <p class="text-sm">
            正在从禅道拉取数据…
          </p>
        </div>

        <Tabs
          v-else
          default-value="tasks"
          class="gap-4"
        >
          <TabsList>
            <TabsTrigger value="tasks">
              我的任务
              <Badge
                v-if="tasks.length"
                variant="secondary"
                class="ml-1"
              >
                {{ tasks.length }}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="bugs">
              我的 Bug
              <Badge
                v-if="bugs.length"
                variant="secondary"
                class="ml-1"
              >
                {{ bugs.length }}
              </Badge>
            </TabsTrigger>
          </TabsList>

          <TabsContent value="tasks">
            <div
              v-if="tasks.length === 0"
              class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
            >
              <p class="text-sm">
                没有进行中的任务
              </p>
              <p class="text-xs">
                休息一下吧
              </p>
            </div>
            <TaskList
              v-else
              :tasks="sortedTasks"
            />
          </TabsContent>

          <TabsContent value="bugs">
            <div
              v-if="bugs.length === 0"
              class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
            >
              <p class="text-sm">
                没有未关闭的 Bug
              </p>
            </div>
            <BugList
              v-else
              :bugs="sortedBugs"
            />
          </TabsContent>
        </Tabs>
      </div>
    </template>
  </div>
</template>

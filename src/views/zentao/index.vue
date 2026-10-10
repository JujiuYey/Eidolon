<script setup lang="ts">
import { Bug as BugIcon, CloudOff, Lightbulb, ListTodo, MessagesSquare, RefreshCw, Settings } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import SagPageHeader from '@/components/sag/sag-page-header/index.vue';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { useZentaoStore } from '@/stores/zentao';
import { getErrorMessage } from '@/utils/helpers';

import BugList from './_components/bug-list.vue';
import StoryList from './_components/story-list.vue';
import TaskList from './_components/task-list.vue';
import { formatTimestamp, sortBugs, sortStories, sortTasks } from './utils/display';

const store = useZentaoStore();
const router = useRouter();

const tasks = computed(() => store.myWork?.tasks ?? []);
const bugs = computed(() => store.myWork?.bugs ?? []);
/** 研发需求（story）与用户需求（requirement）由同一份需求列表按类型拆分 */
const devStories = computed(() => (store.myWork?.stories ?? []).filter(story => story.story_type === 'story'));
const userStories = computed(() => (store.myWork?.stories ?? []).filter(story => story.story_type === 'requirement'));
const hasAccount = computed(() => store.accounts.length > 0);

const sortedTasks = computed(() => sortTasks(tasks.value));
const sortedBugs = computed(() => sortBugs(bugs.value));
const sortedDevStories = computed(() => sortStories(devStories.value));
const sortedUserStories = computed(() => sortStories(userStories.value));

/** 卡片即导航：点哪张卡，下方列表就展示哪类工作项 */
type WorkSection = 'tasks' | 'dev-stories' | 'user-stories' | 'bugs';
const activeSection = ref<WorkSection>('tasks');

const workSections = computed(() => [
  {
    key: 'tasks' as const,
    label: '进行中任务',
    count: tasks.value.length,
    icon: ListTodo,
    tileClass: 'bg-primary/10',
    iconClass: 'text-primary',
  },
  {
    key: 'dev-stories' as const,
    label: '研发需求',
    count: devStories.value.length,
    icon: Lightbulb,
    tileClass: 'bg-primary/10',
    iconClass: 'text-primary',
  },
  {
    key: 'user-stories' as const,
    label: '用户需求',
    count: userStories.value.length,
    icon: MessagesSquare,
    tileClass: 'bg-primary/10',
    iconClass: 'text-primary',
  },
  {
    key: 'bugs' as const,
    label: '未关闭 Bug',
    count: bugs.value.length,
    icon: BugIcon,
    tileClass: 'bg-destructive/10',
    iconClass: 'text-destructive',
  },
]);

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
      `已刷新：${tasks.value.length} 个任务、${bugs.value.length} 个 Bug、` +
        `${devStories.value.length} 个研发需求、${userStories.value.length} 个用户需求`,
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

/** 账户管理统一在应用设置，功能页只负责选择 */
function goSettings() {
  router.push('/app-setting');
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <SagPageHeader class="px-6 pt-6" title="禅道">
      <template #meta>
        <span v-if="store.myWork">更新于 {{ formatTimestamp(store.myWork.fetched_at) }}</span>
      </template>
      <template #actions>
        <template v-if="hasAccount">
          <Select :model-value="store.activeAccountId" @update:model-value="handleAccountChange">
            <SelectTrigger class="w-52">
              <SelectValue placeholder="选择账户" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="account of store.accounts" :key="account.id" :value="account.id">
                {{ account.name }}（{{ account.account }}）
              </SelectItem>
            </SelectContent>
          </Select>

          <Button size="sm" :disabled="store.workLoading || !store.activeAccountId" @click="refreshWork()">
            <RefreshCw class="mr-1 h-4 w-4" :class="store.workLoading && 'animate-spin'" />
            {{ store.workLoading ? '拉取中…' : '刷新' }}
          </Button>
        </template>

        <Button v-else variant="outline" size="sm" @click="goSettings">
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
        <p class="text-sm font-medium">还没有配置禅道账户</p>
        <p class="max-w-sm text-xs text-muted-foreground">
          在应用设置中添加禅道站点与登录账号后，即可在这里查看名下的任务、Bug 与需求。
        </p>
      </div>
      <Button size="sm" @click="goSettings">
        <Settings class="mr-1 h-4 w-4" />
        前往应用设置
      </Button>
    </div>

    <template v-else>
      <!-- 统计卡即导航：点击切换下方列表 -->
      <div v-if="store.myWork" class="grid grid-cols-2 gap-3 px-6 pt-4 sm:grid-cols-4">
        <button
          v-for="section of workSections"
          :key="section.key"
          type="button"
          class="rounded-xl text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          :aria-pressed="activeSection === section.key"
          @click="activeSection = section.key"
        >
          <Card
            class="py-4 transition-colors"
            :class="activeSection === section.key ? 'border-primary/50 bg-primary/5' : 'hover:border-ring/40'"
          >
            <CardContent class="flex items-center gap-3 px-4">
              <div class="flex size-9 items-center justify-center rounded-md" :class="section.tileClass">
                <component :is="section.icon" class="size-4.5" :class="section.iconClass" />
              </div>
              <div>
                <p class="text-xl leading-none font-semibold tabular-nums">
                  {{ section.count }}
                </p>
                <p class="mt-1 text-xs text-muted-foreground">{{ section.label }}</p>
              </div>
            </CardContent>
          </Card>
        </button>
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
            <p class="text-sm font-medium">禅道数据拉取失败</p>
            <p class="max-w-md text-xs leading-5 text-muted-foreground">
              {{ store.workError }}
            </p>
          </div>
          <Button size="sm" variant="outline" :disabled="store.workLoading" @click="refreshWork(false)">
            <RefreshCw class="mr-1 h-4 w-4" :class="store.workLoading && 'animate-spin'" />
            重试
          </Button>
        </div>

        <div
          v-else-if="store.workLoading && !store.myWork"
          class="flex flex-col items-center justify-center gap-2 py-20 text-muted-foreground"
        >
          <RefreshCw class="size-5 animate-spin" />
          <p class="text-sm">正在从禅道拉取数据…</p>
        </div>

        <template v-if="activeSection === 'tasks'">
          <div
            v-if="tasks.length === 0"
            class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
          >
            <p class="text-sm">没有进行中的任务</p>
            <p class="text-xs">休息一下吧</p>
          </div>
          <TaskList v-else :tasks="sortedTasks" />
        </template>

        <template v-else-if="activeSection === 'dev-stories'">
          <div
            v-if="devStories.length === 0"
            class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
          >
            <p class="text-sm">没有指派给你的研发需求</p>
          </div>
          <StoryList v-else :stories="sortedDevStories" />
        </template>

        <template v-else-if="activeSection === 'user-stories'">
          <div
            v-if="userStories.length === 0"
            class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
          >
            <p class="text-sm">没有指派给你的用户需求</p>
          </div>
          <StoryList v-else :stories="sortedUserStories" />
        </template>

        <template v-else>
          <div
            v-if="bugs.length === 0"
            class="flex flex-col items-center justify-center gap-1 py-20 text-muted-foreground"
          >
            <p class="text-sm">没有未关闭的 Bug</p>
          </div>
          <BugList v-else :bugs="sortedBugs" />
        </template>
      </div>
    </template>
  </div>
</template>

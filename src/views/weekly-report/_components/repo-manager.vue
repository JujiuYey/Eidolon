<script setup lang="ts">
import type { WeeklyReportRepo } from '@/types/weekly-report';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderGit2, Plus, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';

interface Props {
  repos: WeeklyReportRepo[];
  isLoading: boolean;
}

interface Emits {
  (e: 'add', paths: string[]): void;
  (e: 'remove', repoId: string): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();

const isPicking = ref(false);
const removeOpen = ref(false);
const pendingRepo = ref<WeeklyReportRepo | null>(null);

async function pickRepos() {
  isPicking.value = true;
  try {
    const selected = await open({ directory: true, multiple: true, title: '选择 git 仓库目录' });
    if (!selected) {
      return;
    }
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length > 0) {
      emit('add', paths);
    }
  } finally {
    isPicking.value = false;
  }
}

function askRemove(repo: WeeklyReportRepo) {
  pendingRepo.value = repo;
  removeOpen.value = true;
}

function confirmRemove() {
  if (pendingRepo.value) {
    emit('remove', pendingRepo.value.id);
  }
  removeOpen.value = false;
  pendingRepo.value = null;
}
</script>

<template>
  <div class="flex min-h-0 flex-col rounded-xl border bg-card">
    <div class="flex items-center justify-between gap-2 px-4 py-3">
      <h2 class="flex items-center gap-2 text-sm font-semibold">
        <FolderGit2 class="h-4 w-4 text-primary" />
        代码仓库
      </h2>
      <Button
        size="sm"
        variant="outline"
        :disabled="isPicking"
        @click="pickRepos"
      >
        <Plus class="h-4 w-4" />
        添加
      </Button>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-1 px-2 pb-2">
        <template v-if="isLoading">
          <Skeleton
            v-for="index of 3"
            :key="index"
            class="mx-2 h-12"
          />
        </template>

        <p
          v-else-if="repos.length === 0"
          class="px-2 py-6 text-center text-xs text-muted-foreground"
        >
          还没有仓库，点击「添加」选择本地 git 仓库目录。
        </p>

        <template v-else>
          <div
            v-for="repo of repos"
            :key="repo.id"
            class="group flex items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-muted/60"
          >
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium">
                {{ repo.name }}
              </p>
              <p
                class="truncate text-xs text-muted-foreground"
                :title="repo.path"
              >
                {{ repo.path }}
              </p>
            </div>
            <Button
              aria-label="移除仓库"
              class="opacity-0 transition group-hover:opacity-100"
              size="icon"
              variant="ghost"
              @click="askRemove(repo)"
            >
              <Trash2 class="h-4 w-4 text-muted-foreground hover:text-destructive" />
            </Button>
          </div>
        </template>
      </div>
    </ScrollArea>

    <SagConfirm
      v-model:open="removeOpen"
      :description="pendingRepo
        ? `将从周报统计中移除「${pendingRepo.name}」。不会删除磁盘上的仓库文件。`
        : ''"
      title="移除仓库"
      type="destructive"
      @confirm="confirmRemove"
    />
  </div>
</template>

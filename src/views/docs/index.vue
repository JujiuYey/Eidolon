<script setup lang="ts">
import type { DocsEntry } from '@/types/docs';
import { FolderOpen, Save, Sparkles } from 'lucide-vue-next';
import { open } from '@tauri-apps/plugin-dialog';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import SagRichEditor from '@/components/sag/sag-rich-editor/index.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';
import { useDocsStore } from '@/stores/docs';
import { getErrorMessage } from '@/utils/helpers';
import DocsAiPanel from './_components/docs-ai-panel.vue';
import DocsSearchResults from './_components/docs-search-results.vue';
import DocsTree from './_components/docs-tree.vue';
import EntryNameDialog from './_components/entry-name-dialog.vue';

const store = useDocsStore();

// ===== 搜索（300ms 防抖） =====
const searchKeyword = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchKeyword, value => {
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  searchTimer = setTimeout(() => {
    void store.runSearch(value).catch(error => toast.error(getErrorMessage(error, '搜索失败')));
  }, 300);
});

// ===== 文件切换的脏状态守卫 =====
const unsavedOpen = ref(false);
const pendingPath = ref('');

async function openPath(path: string) {
  await store.openFile(path);
}

function handleSelectFile(path: string) {
  if (path === store.activePath) {
    return;
  }
  if (store.isDirty) {
    pendingPath.value = path;
    unsavedOpen.value = true;
    return;
  }
  void openPath(path);
}

function confirmDiscardAndOpen() {
  store.discardChanges();
  const path = pendingPath.value;
  unsavedOpen.value = false;
  pendingPath.value = '';
  if (path) {
    void openPath(path);
  }
}

// ===== 保存（按钮 + Cmd/Ctrl+S） =====
async function handleSave() {
  try {
    const ok = await store.save();
    if (ok) {
      toast.success('已保存');
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    void handleSave();
  }
}

// ===== 新建 / 重命名（共用一个名称输入对话框） =====
interface NameDialogState {
  mode: 'create-file' | 'create-dir' | 'rename';
  targetPath: string;
  initialName: string;
  title: string;
  description: string;
  confirmLabel: string;
}

const nameDialog = ref<NameDialogState | null>(null);

function askCreateFile(dirPath: string) {
  nameDialog.value = {
    mode: 'create-file',
    targetPath: dirPath,
    initialName: '',
    title: '新建文档',
    description: dirPath ? `将在「${dirPath}」下创建，名称会自动补上 .md。` : '在文档库根目录创建，名称会自动补上 .md。',
    confirmLabel: '创建',
  };
}

function askCreateDir(parentPath: string) {
  nameDialog.value = {
    mode: 'create-dir',
    targetPath: parentPath,
    initialName: '',
    title: '新建文件夹',
    description: '用来归类文档，可以嵌套。',
    confirmLabel: '创建',
  };
}

function askRename(entry: DocsEntry) {
  nameDialog.value = {
    mode: 'rename',
    targetPath: entry.path,
    initialName: entry.is_dir ? entry.name : entry.name.replace(/\.md$/i, ''),
    title: entry.is_dir ? '重命名文件夹' : '重命名文档',
    description: entry.is_dir
      ? '文件夹内的文档会跟着一起改路径。'
      : '不需要写 .md 后缀，会自动保留。',
    confirmLabel: '保存',
  };
}

async function confirmNameDialog(name: string) {
  const dialog = nameDialog.value;
  if (!dialog) {
    return;
  }
  nameDialog.value = null;

  try {
    if (dialog.mode === 'create-file') {
      await store.createFile(dialog.targetPath, name);
    } else if (dialog.mode === 'create-dir') {
      await store.createDirectory(dialog.targetPath, name);
    } else {
      await store.renameEntry(dialog.targetPath, name);
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '操作失败'));
  }
}

// ===== 删除 =====
const removeOpen = ref(false);
const pendingRemove = ref<DocsEntry | null>(null);

function askRemove(entry: DocsEntry) {
  pendingRemove.value = entry;
  removeOpen.value = true;
}

async function confirmRemove() {
  const entry = pendingRemove.value;
  removeOpen.value = false;
  pendingRemove.value = null;
  if (!entry) {
    return;
  }

  try {
    await store.removeEntry(entry.path);
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

// ===== 文档库目录 =====
const isPickingRoot = ref(false);

async function pickRootDir() {
  isPickingRoot.value = true;
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择文档库目录',
    });
    if (typeof selected === 'string' && selected) {
      await store.setRootDir(selected);
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '设置文档库目录失败'));
  } finally {
    isPickingRoot.value = false;
  }
}

// ===== AI 助手面板（应用 / 撤销 都只改内存内容，保存仍走 ⌘S） =====
const aiOpen = ref(false);

function handleAiApply(content: string) {
  store.content = content;
  toast.success('已应用到文档，⌘S 保存后写入磁盘');
}

function handleAiRestore(content: string) {
  store.content = content;
  toast.success('已恢复应用前的版本');
}

// ===== 派生展示 =====
const isSearching = computed(() => searchKeyword.value.trim().length > 0);

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
  void store.initialize().catch(error => toast.error(getErrorMessage(error, '文档库加载失败')));
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown);
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <div class="flex min-h-0 flex-1 gap-4 px-6 py-4">
      <aside class="flex w-72 shrink-0 flex-col gap-3">
        <Input
          v-model="searchKeyword"
          aria-label="搜索文档"
          class="h-9 shrink-0"
          placeholder="搜索文档内容"
        />
        <div class="min-h-0 flex-1">
          <DocsSearchResults
            v-if="isSearching"
            :is-searching="store.searching"
            :keyword="searchKeyword.trim()"
            :results="store.searchResults"
            @select="handleSelectFile"
          />
          <DocsTree
            v-else
            :active-path="store.activePath"
            :entries="store.entries"
            :is-loading="store.loading"
            @change-root="pickRootDir"
            @create-dir="askCreateDir"
            @create-file="askCreateFile"
            @remove="askRemove"
            @rename="askRename"
            @select="handleSelectFile"
          />
        </div>
      </aside>

      <!-- 未设置文档库目录 -->
      <div
        v-if="!store.rootDir"
        class="flex min-w-0 flex-1 flex-col items-center justify-center gap-4 rounded-xl border bg-card text-center"
      >
        <div class="flex h-11 w-11 items-center justify-center rounded-full border">
          <FolderOpen class="h-5 w-5 text-primary" />
        </div>
        <div>
          <p class="text-sm font-semibold">
            设置你的文档库
          </p>
          <p class="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
            选一个存放 .md 文件的文件夹，里面的文档会全部出现在这里。
            文档始终是磁盘上的普通文件，随时可以用其他工具打开。
          </p>
        </div>
        <Button
          :disabled="isPickingRoot"
          @click="pickRootDir"
        >
          <FolderOpen class="h-4 w-4" />
          选择文档库目录
        </Button>
      </div>

      <!-- 已设置但没打开文档 -->
      <div
        v-else-if="!store.activePath"
        class="flex min-w-0 flex-1 items-center justify-center rounded-xl border bg-card"
      >
        <p class="text-sm text-muted-foreground">
          从左侧选择一篇文档，或点「新建」开始写。
        </p>
      </div>

      <!-- 编辑区 -->
      <div
        v-else
        class="flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card"
      >
        <p
          v-if="store.fileError"
          class="shrink-0 border-b px-4 py-2 text-sm text-destructive"
        >
          {{ store.fileError }}
        </p>
        <SagRichEditor v-model="store.content">
          <template #actions>
            <Button
              :aria-label="aiOpen ? '关闭 AI 助手' : '打开 AI 助手'"
              class="h-8 w-8" :class="[aiOpen ? 'text-primary' : '']"
              :title="aiOpen ? '关闭 AI 助手' : '打开 AI 助手'"
              :variant="aiOpen ? 'secondary' : 'ghost'"
              size="icon"
              @click="aiOpen = !aiOpen"
            >
              <Sparkles class="h-4 w-4" />
            </Button>
            <span
              v-if="store.isDirty"
              class="flex shrink-0 items-center gap-1.5 text-xs text-amber-600 dark:text-amber-400"
            >
              <span class="h-1.5 w-1.5 rounded-full bg-current" />
              未保存
            </span>
            <Button
              class="h-8 shrink-0"
              :disabled="!store.isDirty || store.saving"
              size="sm"
              @click="handleSave"
            >
              <Spinner
                v-if="store.saving"
                class="h-4 w-4"
              />
              <Save
                v-else
                class="h-4 w-4"
              />
              保存
            </Button>
          </template>
        </SagRichEditor>
      </div>

      <!-- AI 助手面板：编辑区右侧，切换文档时随 :key 重建（每篇文档独立会话） -->
      <DocsAiPanel
        v-if="aiOpen"
        :key="store.activePath"
        :active-path="store.activePath"
        :document="store.content"
        @apply="handleAiApply"
        @close="aiOpen = false"
        @restore="handleAiRestore"
      />
    </div>

    <!-- 未保存守卫 -->
    <SagConfirm
      v-model:open="unsavedOpen"
      :description="store.activeEntry
        ? `当前文档「${store.activeEntry.name}」有未保存的修改，放弃并打开新文档？`
        : '当前文档有未保存的修改，放弃并打开新文档？'"
      title="未保存的修改"
      type="default"
      @confirm="confirmDiscardAndOpen"
    />

    <!-- 删除确认 -->
    <SagConfirm
      v-model:open="removeOpen"
      :description="pendingRemove
        ? pendingRemove.is_dir
          ? `删除空文件夹「${pendingRemove.name}」。`
          : `会直接删除磁盘上的「${pendingRemove.name}」，不可恢复。`
        : ''"
      title="删除"
      type="destructive"
      @confirm="confirmRemove"
    />

    <!-- 新建 / 重命名 -->
    <EntryNameDialog
      :confirm-label="nameDialog?.confirmLabel ?? '创建'"
      :description="nameDialog?.description ?? ''"
      :initial-name="nameDialog?.initialName ?? ''"
      :open="!!nameDialog"
      :title="nameDialog?.title ?? ''"
      @confirm="confirmNameDialog"
      @update:open="(value) => { if (!value) nameDialog = null; }"
    />
  </div>
</template>

<script setup lang="ts">
import { FolderOpen, ListTree, Save, Sparkles, StretchHorizontal } from 'lucide-vue-next';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import ConfirmDialog from '@/components/ConfirmDialog.vue';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import SagRichEditor from '@/components/sag/sag-rich-editor/index.vue';
import type { RichEditorTocItem } from '@/components/sag/sag-rich-editor/toc';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { Spinner } from '@/components/ui/spinner';
import { useConfirm } from '@/composables/use-confirm';
import { onDocsExternalChange } from '@/services/docs';
import { useDocsStore } from '@/stores/docs';
import type { DocsEntry } from '@/types/docs';
import { copyToClipboard, getErrorMessage } from '@/utils/helpers';

import DocsAiPanel from './_components/docs-ai-panel.vue';
import DocsSearchResults from './_components/docs-search-results.vue';
import DocsTocPanel from './_components/docs-toc-panel.vue';
import DocsTree from './_components/docs-tree.vue';
import EntryNameDialog from './_components/entry-name-dialog.vue';
import { docsRootBasename } from './utils/roots';

const props = defineProps<{
  rootId: string;
}>();

const router = useRouter();
const store = useDocsStore();
const confirm = useConfirm();

// ===== 当前文档库加载（由路由参数驱动） =====
const ready = ref(false);

async function bootstrapForRoot() {
  ready.value = false;
  try {
    await store.initialize();
    if (store.activeRootId !== props.rootId) {
      await store.openRoot(props.rootId, { discardUnsaved: true });
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '文档库加载失败'));
  } finally {
    ready.value = true;
  }
}

watch(
  () => props.rootId,
  () => void bootstrapForRoot(),
  { immediate: true },
);

const rootLabel = computed(() => (store.activeRoot ? docsRootBasename(store.activeRoot.path) : '文档'));

// ===== 路由级脏守卫：切库、回管理页、去其他页面都经过这里 =====
onBeforeRouteUpdate(() => guardUnsavedOnRouteChange());
onBeforeRouteLeave(() => guardUnsavedOnRouteChange());

async function guardUnsavedOnRouteChange(): Promise<boolean> {
  if (!store.isDirty) {
    return true;
  }
  const proceed = await confirm.ask({
    title: '当前文档存在未保存修改',
    description: '离开将放弃当前编辑。请先保存或继续编辑。',
    confirmLabel: '放弃修改',
    cancelLabel: '继续编辑',
    destructive: true,
  });
  if (!proceed) {
    return false;
  }
  store.discardChanges();
  return true;
}

// ===== 搜索（300ms 防抖） =====
const searchKeyword = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchKeyword, value => {
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  searchTimer = setTimeout(() => {
    if (!store.activeRootId) {
      return;
    }
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
    description: dirPath
      ? `将在「${dirPath}」下创建，名称会自动补上 .md。`
      : '在文档库根目录创建，名称会自动补上 .md。',
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
    description: entry.is_dir ? '文件夹内的文档会跟着一起改路径。' : '不需要写 .md 后缀，会自动保留。',
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

// ===== 在访达中显示 / 复制路径 =====

async function handleReveal(entry: DocsEntry) {
  try {
    await store.reveal(entry.path);
  } catch (error) {
    toast.error(getErrorMessage(error, '在访达中显示失败'));
  }
}

/** 复制磁盘绝对路径——贴到终端/编辑器里能直接用 */
async function handleCopyPath(entry: DocsEntry) {
  const root = store.activeRoot;
  if (!root) {
    return;
  }
  const absolute = `${root.path.replace(/\/+$/, '')}/${entry.path}`;
  if (await copyToClipboard(absolute)) {
    toast.success('已复制路径');
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

// ===== 阅读辅助:全宽模式（本地记忆） + 目录面板（标题由编辑器抽出） =====
const editorRef = ref<InstanceType<typeof SagRichEditor> | null>(null);
const tocItems = computed<RichEditorTocItem[]>(() => editorRef.value?.tocItems ?? []);
const tocOpen = ref(false);

const FULL_WIDTH_STORAGE_KEY = 'eidolon-docs-full-width';
const fullWidth = ref(localStorage.getItem(FULL_WIDTH_STORAGE_KEY) === '1');

watch(fullWidth, value => {
  localStorage.setItem(FULL_WIDTH_STORAGE_KEY, value ? '1' : '0');
});

function handleTocJump(pos: number) {
  editorRef.value?.jumpToHeading(pos);
}

// ===== 派生展示 =====
const isSearching = computed(() => searchKeyword.value.trim().length > 0);

// ===== 外部变更（MCP agent 写入）：事件 → store 防抖处理 =====
let unlistenExternalChange: (() => void) | null = null;

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
  void onDocsExternalChange(change => store.handleExternalChange(change)).then(unlisten => {
    unlistenExternalChange = unlisten;
  });
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown);
  unlistenExternalChange?.();
  unlistenExternalChange = null;
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- 四栏全部可拖宽（reka Splitter）；autoSaveId 按面板 id 记住每栏宽度 -->
    <div class="flex min-h-0 flex-1 px-6 py-4">
      <ResizablePanelGroup
        id="docs-workspace-group"
        auto-save-id="docs-workspace-layout"
        class="min-h-0 flex-1"
        direction="horizontal"
      >
        <!-- 文档树 -->
        <ResizablePanel
          id="docs-tree-panel"
          class="min-h-0"
          :default-size="18"
          :max-size="30"
          :min-size="14"
          :order="0"
        >
          <aside class="flex h-full min-w-0 flex-col gap-3">
            <Input v-model="searchKeyword" aria-label="搜索文档" class="h-9 shrink-0" placeholder="搜索文档内容" />
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
                :root-id="store.activeRootId"
                :is-loading="store.loading || !ready"
                :title="rootLabel"
                @create-dir="askCreateDir"
                @create-file="askCreateFile"
                @manage="router.push('/docs')"
                @remove="askRemove"
                @rename="askRename"
                @reveal="handleReveal"
                @copy-path="handleCopyPath"
                @select="handleSelectFile"
              />
            </div>
          </aside>
        </ResizablePanel>

        <!-- 拖柄即栏间空隙:平时就是页面底色,悬停/拖动时浮出圆角高亮 -->
        <ResizableHandle
          id="docs-tree-handle"
          class="w-2 cursor-col-resize rounded-md bg-transparent transition-colors duration-150 data-[state=drag]:bg-primary/20 data-[state=hover]:bg-primary/10"
        />

        <!-- 主区：加载 / 库失效 / 未开文档 / 编辑器 四态 -->
        <ResizablePanel id="docs-editor-panel" class="min-h-0" :min-size="25" :order="1">
          <!-- 加载中 -->
          <div v-if="!ready" class="flex h-full items-center justify-center rounded-xl border bg-card">
            <Spinner class="size-6 text-muted-foreground" />
          </div>

          <!-- 文档库不存在（被移除或链接失效） -->
          <div
            v-else-if="!store.activeRoot"
            class="flex h-full flex-col items-center justify-center gap-4 rounded-xl border bg-card text-center"
          >
            <div class="flex h-11 w-11 items-center justify-center rounded-full border">
              <FolderOpen class="h-5 w-5 text-primary" />
            </div>
            <div>
              <p class="text-sm font-semibold">找不到这个文档库</p>
              <p class="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
                它可能已经被移除，或者路径在磁盘上失效了。
              </p>
            </div>
            <Button @click="router.push('/docs')">去管理目录</Button>
          </div>

          <!-- 已打开库但没打开文档 -->
          <div v-else-if="!store.activePath" class="flex h-full items-center justify-center rounded-xl border bg-card">
            <p class="text-sm text-muted-foreground">从左侧选择一篇文档，或点「新建」开始写。</p>
          </div>

          <!-- 编辑区 -->
          <div v-else class="flex h-full min-w-0 flex-col overflow-hidden rounded-xl border bg-card">
            <!-- 外部（MCP agent）改动提示：只在本地有未保存修改时出现 -->
            <div
              v-if="store.externalNotice"
              class="flex shrink-0 items-center justify-between gap-3 border-b bg-amber-500/10 px-4 py-2"
            >
              <p
                v-if="store.externalNotice.kind === 'modified'"
                class="text-sm leading-5 text-amber-700 dark:text-amber-400"
              >
                「{{ store.externalNotice.name }}」刚被外部修改（可能来自 MCP agent）。加载新版本会放弃未保存的修改。
              </p>
              <p v-else class="text-sm leading-5 text-amber-700 dark:text-amber-400">
                「{{ store.externalNotice.name }}」已被外部删除。你的修改还在编辑器里，保存时会重新创建文件。
              </p>
              <div class="flex shrink-0 items-center gap-2">
                <template v-if="store.externalNotice.kind === 'modified'">
                  <Button class="h-7 px-2.5 text-xs" size="sm" variant="outline" @click="store.acceptExternalVersion()">
                    加载新版本
                  </Button>
                  <Button class="h-7 px-2.5 text-xs" size="sm" variant="ghost" @click="store.dismissExternalNotice()">
                    保留我的
                  </Button>
                </template>
                <Button
                  v-else
                  class="h-7 px-2.5 text-xs"
                  size="sm"
                  variant="outline"
                  @click="store.dismissExternalNotice()"
                >
                  知道了
                </Button>
              </div>
            </div>
            <p v-if="store.fileError" class="shrink-0 border-b px-4 py-2 text-sm text-destructive">
              {{ store.fileError }}
            </p>
            <SagRichEditor :key="store.activePath" ref="editorRef" v-model="store.content" :full-width="fullWidth">
              <template #actions>
                <Button
                  :aria-label="fullWidth ? '退出全宽模式' : '进入全宽模式'"
                  :aria-pressed="fullWidth"
                  class="h-8 w-8"
                  :class="[fullWidth ? 'text-primary' : '']"
                  :title="fullWidth ? '退出全宽模式' : '全宽模式'"
                  :variant="fullWidth ? 'secondary' : 'ghost'"
                  size="icon"
                  @click="fullWidth = !fullWidth"
                >
                  <StretchHorizontal class="h-4 w-4" />
                </Button>
                <Button
                  :aria-label="tocOpen ? '关闭目录' : '打开目录'"
                  :aria-pressed="tocOpen"
                  class="h-8 w-8"
                  :class="[tocOpen ? 'text-primary' : '']"
                  :title="tocOpen ? '关闭目录' : '目录'"
                  :variant="tocOpen ? 'secondary' : 'ghost'"
                  size="icon"
                  @click="tocOpen = !tocOpen"
                >
                  <ListTree class="h-4 w-4" />
                </Button>
                <Button
                  :aria-label="aiOpen ? '关闭 AI 助手' : '打开 AI 助手'"
                  class="h-8 w-8"
                  :class="[aiOpen ? 'text-primary' : '']"
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
                <Button class="h-8 shrink-0" :disabled="!store.isDirty || store.saving" size="sm" @click="handleSave">
                  <Spinner v-if="store.saving" class="h-4 w-4" />
                  <Save v-else class="h-4 w-4" />
                  保存
                </Button>
              </template>
            </SagRichEditor>
          </div>
        </ResizablePanel>

        <!-- 目录面板:点击标题平滑跳到对应位置 -->
        <template v-if="tocOpen && store.activePath">
          <ResizableHandle
            id="docs-toc-handle"
            class="w-2 cursor-col-resize rounded-md bg-transparent transition-colors duration-150 data-[state=drag]:bg-primary/20 data-[state=hover]:bg-primary/10"
          />
          <ResizablePanel
            id="docs-toc-panel"
            class="min-h-0"
            :default-size="16"
            :max-size="30"
            :min-size="12"
            :order="2"
          >
            <DocsTocPanel :items="tocItems" @jump="handleTocJump" />
          </ResizablePanel>
        </template>

        <!-- AI 助手面板：会话按文档保存在组件内部，无需随切换重建 -->
        <template v-if="aiOpen">
          <ResizableHandle
            id="docs-ai-handle"
            class="w-2 cursor-col-resize rounded-md bg-transparent transition-colors duration-150 data-[state=drag]:bg-primary/20 data-[state=hover]:bg-primary/10"
          />
          <ResizablePanel
            id="docs-ai-panel"
            class="min-h-0"
            :default-size="21"
            :max-size="35"
            :min-size="15"
            :order="3"
          >
            <DocsAiPanel
              :active-path="store.activePath"
              :document="store.content"
              @apply="handleAiApply"
              @close="aiOpen = false"
              @restore="handleAiRestore"
            />
          </ResizablePanel>
        </template>
      </ResizablePanelGroup>
    </div>

    <!-- 未保存守卫（库内切换文档） -->
    <SagConfirm
      v-model:open="unsavedOpen"
      :description="
        store.activeEntry
          ? `当前文档「${store.activeEntry.name}」有未保存的修改，放弃并打开新文档？`
          : '当前文档有未保存的修改，放弃并打开新文档？'
      "
      title="未保存的修改"
      type="default"
      @confirm="confirmDiscardAndOpen"
    />

    <!-- 删除确认 -->
    <SagConfirm
      v-model:open="removeOpen"
      :description="
        pendingRemove
          ? pendingRemove.is_dir
            ? `删除空文件夹「${pendingRemove.name}」。`
            : `会直接删除磁盘上的「${pendingRemove.name}」，不可恢复。`
          : ''
      "
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
      @update:open="
        value => {
          if (!value) nameDialog = null;
        }
      "
    />

    <!-- 路由切换的未保存守卫 -->
    <ConfirmDialog
      :open="confirm.state.open"
      @cancel="confirm.onCancel"
      @confirm="confirm.onConfirm"
      @update:open="confirm.onOpenChange"
    />
  </div>
</template>

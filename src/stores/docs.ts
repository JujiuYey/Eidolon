import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import * as docsServiceApi from '@/services/docs';
import type { DocsEntry, DocsRoot, DocsSearchResult, DocsSettings } from '@/types/docs';
import { buildDocsTree, firstLevelDirPaths } from '@/views/docs/utils/tree';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface DocsService {
  getDocsSettings: () => Promise<DocsSettings>;
  upsertDocsSettings: (settings: DocsSettings) => Promise<DocsSettings>;
  listDocsEntries: (rootId: string) => Promise<DocsEntry[]>;
  readDocsFile: (rootId: string, path: string) => Promise<string>;
  saveDocsFile: (rootId: string, path: string, content: string) => Promise<DocsEntry>;
  createDocsFile: (rootId: string, dirPath: string, name: string) => Promise<DocsEntry>;
  createDocsDirectory: (rootId: string, parentPath: string, name: string) => Promise<DocsEntry>;
  renameDocsEntry: (rootId: string, path: string, newName: string) => Promise<DocsEntry>;
  deleteDocsEntry: (rootId: string, path: string) => Promise<string>;
  searchDocs: (rootId: string, keyword: string) => Promise<DocsSearchResult[]>;
}

const defaultService: DocsService = docsServiceApi;

/**
 * 文档管理 store（工厂模式，便于测试注入 fake service）。
 *
 * 支持多个文档库（roots）：entries/编辑态只描述当前激活的库，切库用
 * openRoot（脏且有未确认时抛错，由视图层 confirm 后带 discardUnsaved
 * 重试）。文档是磁盘上的 .md 文件：entries 只是文件系统快照，任何写
 * 操作后重新拉取。保存是显式动作（Cmd+S / 按钮）。
 */
export function createDocsStore(options: { service?: DocsService } = {}) {
  const service = options.service ?? defaultService;

  const roots = ref<DocsRoot[]>([]);
  const activeRootId = ref('');
  const entries = ref<DocsEntry[]>([]);
  const loading = ref(false);

  const activePath = ref('');
  const content = ref('');
  const savedContent = ref('');
  const fileError = ref('');

  const saving = ref(false);

  const searchResults = ref<DocsSearchResult[]>([]);
  const searching = ref(false);

  const tree = computed(() => buildDocsTree(entries.value));
  const defaultExpandedPaths = computed(() => firstLevelDirPaths(entries.value));
  const activeEntry = computed(
    () => entries.value.find(entry => entry.path === activePath.value && !entry.is_dir) ?? null,
  );
  const isDirty = computed(() => content.value !== savedContent.value);
  const activeRoot = computed(() => roots.value.find(root => root.id === activeRootId.value) ?? null);

  /** 只拉文档库列表（侧边栏与管理页用），entries 由工作区按库加载 */
  async function initialize() {
    const settings = await service.getDocsSettings();
    roots.value = settings.roots;
    if (activeRootId.value && !settings.roots.some(root => root.id === activeRootId.value)) {
      activeRootId.value = '';
      resetActive();
      entries.value = [];
    }
  }

  async function loadEntries() {
    if (!activeRootId.value) {
      return;
    }
    loading.value = true;
    try {
      entries.value = await service.listDocsEntries(activeRootId.value);
    } finally {
      loading.value = false;
    }
  }

  /** 切换文档库。有未保存修改且未带 discardUnsaved 时抛错，由视图层先 confirm */
  async function openRoot(rootId: string, { discardUnsaved = false } = {}) {
    if (activeRootId.value === rootId) {
      return;
    }
    if (isDirty.value && !discardUnsaved) {
      throw new Error('当前文档存在未保存修改');
    }

    activeRootId.value = rootId;
    resetActive();
    clearSearch();
    await loadEntries();
    // 默认打开该库第一篇文档，空库则停在空态
    const firstFile = entries.value.find(entry => !entry.is_dir);
    if (firstFile) {
      await openFile(firstFile.path);
    }
  }

  /** 保存文档库列表（添加/移除后由管理页调用），返回后端校验去重后的结果 */
  async function saveRoots(nextRoots: DocsRoot[]): Promise<DocsRoot[]> {
    const saved = await service.upsertDocsSettings({ root_dir: null, roots: nextRoots });
    roots.value = saved.roots;
    if (activeRootId.value && !saved.roots.some(root => root.id === activeRootId.value)) {
      activeRootId.value = '';
      resetActive();
      entries.value = [];
    }
    return saved.roots;
  }

  async function openFile(path: string) {
    if (activePath.value === path) {
      return;
    }

    fileError.value = '';
    try {
      const text = await service.readDocsFile(activeRootId.value, path);
      activePath.value = path;
      content.value = text;
      savedContent.value = text;
    } catch (error) {
      // 读取失败也切换过去，让用户看到错误态而不是无声无息
      activePath.value = path;
      content.value = '';
      savedContent.value = '';
      fileError.value = error instanceof Error ? error.message : String(error);
    }
  }

  /** 显式保存（视图的 Cmd+S / 保存按钮）。返回是否成功 */
  async function save(): Promise<boolean> {
    if (!activeRootId.value || !activePath.value || !isDirty.value || saving.value) {
      return false;
    }

    saving.value = true;
    try {
      await service.saveDocsFile(activeRootId.value, activePath.value, content.value);
      savedContent.value = content.value;
      await loadEntries();
      return true;
    } finally {
      saving.value = false;
    }
  }

  function discardChanges() {
    content.value = savedContent.value;
  }

  async function createFile(dirPath: string, name: string) {
    const entry = await service.createDocsFile(activeRootId.value, dirPath, name);
    await loadEntries();
    await openFile(entry.path);
    return entry;
  }

  async function createDirectory(parentPath: string, name: string) {
    const entry = await service.createDocsDirectory(activeRootId.value, parentPath, name);
    await loadEntries();
    return entry;
  }

  async function renameEntry(path: string, newName: string) {
    const renamed = await service.renameDocsEntry(activeRootId.value, path, newName);
    await loadEntries();
    // 重命名的是当前打开的文档时，同步活动路径（内容不变）
    if (activePath.value === path) {
      activePath.value = renamed.path;
    }
    return renamed;
  }

  async function removeEntry(path: string) {
    await service.deleteDocsEntry(activeRootId.value, path);
    await loadEntries();
    if (activePath.value === path) {
      resetActive();
      const firstFile = entries.value.find(entry => !entry.is_dir);
      if (firstFile) {
        await openFile(firstFile.path);
      }
    }
  }

  async function runSearch(keyword: string) {
    const trimmed = keyword.trim();
    if (!trimmed) {
      clearSearch();
      return;
    }

    searching.value = true;
    try {
      searchResults.value = await service.searchDocs(activeRootId.value, trimmed);
    } finally {
      searching.value = false;
    }
  }

  function clearSearch() {
    searchResults.value = [];
  }

  function resetActive() {
    activePath.value = '';
    content.value = '';
    savedContent.value = '';
    fileError.value = '';
  }

  return {
    // 状态
    roots,
    activeRootId,
    activeRoot,
    entries,
    loading,
    activePath,
    content,
    fileError,
    saving,
    searchResults,
    searching,
    // 派生
    tree,
    defaultExpandedPaths,
    activeEntry,
    isDirty,
    // 动作
    initialize,
    openRoot,
    saveRoots,
    loadEntries,
    openFile,
    save,
    discardChanges,
    createFile,
    createDirectory,
    renameEntry,
    removeEntry,
    runSearch,
    clearSearch,
  };
}

export type DocsStore = ReturnType<typeof createDocsStore>;

export const useDocsStore = defineStore('docs', () => createDocsStore());

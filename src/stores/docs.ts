import type { DocsEntry, DocsSearchResult, DocsSettings } from '@/types/docs';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as docsServiceApi from '@/services/docs';
import { buildDocsTree, firstLevelDirPaths } from '@/views/docs/utils/tree';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface DocsService {
  getDocsSettings: () => Promise<DocsSettings>;
  upsertDocsSettings: (settings: DocsSettings) => Promise<DocsSettings>;
  listDocsEntries: () => Promise<DocsEntry[]>;
  readDocsFile: (path: string) => Promise<string>;
  saveDocsFile: (path: string, content: string) => Promise<DocsEntry>;
  createDocsFile: (dirPath: string, name: string) => Promise<DocsEntry>;
  createDocsDirectory: (parentPath: string, name: string) => Promise<DocsEntry>;
  renameDocsEntry: (path: string, newName: string) => Promise<DocsEntry>;
  deleteDocsEntry: (path: string) => Promise<string>;
  searchDocs: (keyword: string) => Promise<DocsSearchResult[]>;
}

const defaultService: DocsService = docsServiceApi;

/**
 * 文档管理 store（工厂模式，便于测试注入 fake service）。
 *
 * 文档是磁盘上的 .md 文件：entries 只是文件系统的快照，任何写操作后
 * 重新拉取。编辑用脏状态跟踪（savedContent 快照），保存是显式动作
 * （Cmd+S / 按钮），切换文档前的守卫由视图层 confirm 承担。
 */
export function createDocsStore(options: { service?: DocsService } = {}) {
  const service = options.service ?? defaultService;

  const rootDir = ref<string | null>(null);
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

  async function loadEntries() {
    loading.value = true;
    try {
      entries.value = await service.listDocsEntries();
    } finally {
      loading.value = false;
    }
  }

  async function initialize() {
    const [settings] = await Promise.all([service.getDocsSettings(), loadEntries()]);
    rootDir.value = settings.root_dir;
    // 默认打开第一篇文档，空库则停在空态
    const firstFile = entries.value.find(entry => !entry.is_dir);
    if (firstFile) {
      await openFile(firstFile.path);
    }
  }

  async function setRootDir(path: string | null) {
    const saved = await service.upsertDocsSettings({ root_dir: path });
    rootDir.value = saved.root_dir;
    resetActive();
    await loadEntries();
    const firstFile = entries.value.find(entry => !entry.is_dir);
    if (firstFile) {
      await openFile(firstFile.path);
    }
  }

  async function openFile(path: string) {
    if (activePath.value === path) {
      return;
    }

    fileError.value = '';
    try {
      const text = await service.readDocsFile(path);
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
    if (!activePath.value || !isDirty.value || saving.value) {
      return false;
    }

    saving.value = true;
    try {
      await service.saveDocsFile(activePath.value, content.value);
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
    const entry = await service.createDocsFile(dirPath, name);
    await loadEntries();
    await openFile(entry.path);
    return entry;
  }

  async function createDirectory(parentPath: string, name: string) {
    const entry = await service.createDocsDirectory(parentPath, name);
    await loadEntries();
    return entry;
  }

  async function renameEntry(path: string, newName: string) {
    const renamed = await service.renameDocsEntry(path, newName);
    await loadEntries();
    // 重命名的是当前打开的文档时，同步活动路径（内容不变）
    if (activePath.value === path) {
      activePath.value = renamed.path;
    }
    return renamed;
  }

  async function removeEntry(path: string) {
    await service.deleteDocsEntry(path);
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
      searchResults.value = await service.searchDocs(trimmed);
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
    rootDir,
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
    setRootDir,
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

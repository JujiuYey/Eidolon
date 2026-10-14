import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import * as docsServiceApi from '@/services/docs';
import type { DocsEntry, DocsExternalChange, DocsRoot, DocsSearchResult, DocsSettings } from '@/types/docs';
import { buildDocsTree, firstLevelDirPaths, sortEntries } from '@/views/docs/utils/tree';

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
  revealDocsEntry: (rootId: string, path: string) => Promise<void>;
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

  /**
   * 外部（MCP agent）改动了打开中的文档且本地有未保存修改时的提示。
   * modified=被改写（可选加载新版本）；deleted=被删除（保存可重建）。
   * 本地干净时不提示，由 flushExternalChanges 静默跟进磁盘。
   */
  const externalNotice = ref<{ kind: 'modified' | 'deleted'; name: string } | null>(null);

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
      entries.value = sortEntries(await service.listDocsEntries(activeRootId.value));
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
    externalNotice.value = null;
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
      // 保存即以本地版本为准，外部修改提示随之失效
      externalNotice.value = null;
      await loadEntries();
      return true;
    } finally {
      saving.value = false;
    }
  }

  function discardChanges() {
    content.value = savedContent.value;
  }

  // 新建/重命名/删除只就地改这一个已知的条目，不再重新递归扫描整个文档库：
  // 那条路径要遍历磁盘上每个文件，与「新建一个空文件」本身无关。
  async function createFile(dirPath: string, name: string) {
    const entry = await service.createDocsFile(activeRootId.value, dirPath, name);
    entries.value = sortEntries([...entries.value, entry]);
    await openFile(entry.path);
    return entry;
  }

  async function createDirectory(parentPath: string, name: string) {
    const entry = await service.createDocsDirectory(activeRootId.value, parentPath, name);
    entries.value = sortEntries([...entries.value, entry]);
    return entry;
  }

  async function renameEntry(path: string, newName: string) {
    const renamed = await service.renameDocsEntry(activeRootId.value, path, newName);
    // 目录改名会带走整棵子树，路径前缀相同的条目一并改名
    entries.value = sortEntries(
      entries.value.map(entry => {
        if (entry.path === path) {
          return renamed;
        }
        if (entry.path.startsWith(`${path}/`)) {
          return { ...entry, path: renamed.path + entry.path.slice(path.length), name: renamed.name };
        }
        return entry;
      }),
    );
    // 重命名的是当前打开的文档时，同步活动路径（内容不变）
    if (activePath.value === path) {
      activePath.value = renamed.path;
    }
    return renamed;
  }

  async function removeEntry(path: string) {
    await service.deleteDocsEntry(activeRootId.value, path);
    entries.value = entries.value.filter(entry => entry.path !== path && !entry.path.startsWith(`${path}/`));
    if (activePath.value === path) {
      resetActive();
      const firstFile = entries.value.find(entry => !entry.is_dir);
      if (firstFile) {
        await openFile(firstFile.path);
      }
    }
  }

  /** 在系统文件管理器中定位（访达里选中） */
  async function reveal(path: string) {
    await service.revealDocsEntry(activeRootId.value, path);
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

  // ===== 外部变更（MCP agent 写入）：合并防抖窗口内的连续事件后统一处理 =====

  const EXTERNAL_CHANGE_DEBOUNCE_MS = 400;
  let externalTimer: ReturnType<typeof setTimeout> | null = null;
  const pendingExternalChanges = new Map<string, DocsExternalChange>();
  let pendingExternalRoot = '';

  function pathBasename(path: string): string {
    return path.slice(path.lastIndexOf('/') + 1);
  }

  /** 工作区挂载后由 Tauri 事件驱动（见 services/docs.ts::onDocsExternalChange） */
  function handleExternalChange(change: DocsExternalChange) {
    if (change.root_id !== activeRootId.value) {
      return;
    }
    pendingExternalRoot = change.root_id;
    // 同一文件多次变更只留最后一次（事件有序，最后一次即磁盘最终态）
    pendingExternalChanges.set(change.path, change);
    if (externalTimer) {
      clearTimeout(externalTimer);
    }
    externalTimer = setTimeout(() => void flushExternalChanges(), EXTERNAL_CHANGE_DEBOUNCE_MS);
  }

  async function flushExternalChanges() {
    externalTimer = null;
    const changes = [...pendingExternalChanges.values()];
    pendingExternalChanges.clear();
    const rootId = pendingExternalRoot;
    pendingExternalRoot = '';
    if (changes.length === 0 || rootId !== activeRootId.value) {
      return;
    }

    // 先刷文件树，agent 新建/删除的文件立即可见
    await loadEntries();

    const current = changes.find(change => change.path === activePath.value);
    if (!current) {
      return;
    }
    // 防抖等待期间用户可能已切走文档
    if (activePath.value !== current.path) {
      return;
    }

    if (current.action === 'delete') {
      if (!isDirty.value) {
        resetActive();
        const firstFile = entries.value.find(entry => !entry.is_dir);
        if (firstFile) {
          await openFile(firstFile.path);
        }
      } else {
        // 保留编辑内容：保存时会重新创建文件
        externalNotice.value = { kind: 'deleted', name: pathBasename(current.path) };
      }
      return;
    }

    if (!isDirty.value) {
      // 本地干净时静默跟进磁盘最新内容
      try {
        const text = await service.readDocsFile(rootId, current.path);
        if (activePath.value === current.path) {
          content.value = text;
          savedContent.value = text;
        }
      } catch {
        // 读取失败（如随即又被删）留待下一次事件修正
      }
    } else {
      externalNotice.value = {
        kind: 'modified',
        name: activeEntry.value?.name ?? pathBasename(current.path),
      };
    }
  }

  /** 提示条「加载新版本」：放弃本地未保存修改，跟进磁盘内容 */
  async function acceptExternalVersion() {
    externalNotice.value = null;
    const rootId = activeRootId.value;
    const path = activePath.value;
    if (!rootId || !path) {
      return;
    }
    try {
      const text = await service.readDocsFile(rootId, path);
      if (activePath.value === path) {
        content.value = text;
        savedContent.value = text;
      }
    } catch (error) {
      fileError.value = error instanceof Error ? error.message : String(error);
    }
  }

  /** 提示条「保留我的」/「知道了」：只关提示，内容不动 */
  function dismissExternalNotice() {
    externalNotice.value = null;
  }

  function resetActive() {
    activePath.value = '';
    content.value = '';
    savedContent.value = '';
    fileError.value = '';
    externalNotice.value = null;
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
    externalNotice,
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
    reveal,
    renameEntry,
    removeEntry,
    runSearch,
    clearSearch,
    handleExternalChange,
    acceptExternalVersion,
    dismissExternalNotice,
  };
}

export type DocsStore = ReturnType<typeof createDocsStore>;

export const useDocsStore = defineStore('docs', () => createDocsStore());

import { createPinia, defineStore, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createDocsStore } from '@/stores/docs';
import type { DocsService } from '@/stores/docs';
import type { DocsEntry, DocsRoot, DocsSearchResult } from '@/types/docs';

const ROOT_A: DocsRoot = { id: 'root-a', path: '/tmp/docs-a' };
const ROOT_B: DocsRoot = { id: 'root-b', path: '/tmp/docs-b' };

function bindStore(service: DocsService) {
  const useBound = defineStore('docs-test', () => createDocsStore({ service }));
  return useBound();
}

function makeEntry(overrides: Partial<DocsEntry> & Pick<DocsEntry, 'name' | 'path' | 'is_dir'>): DocsEntry {
  return { size: 10, updated_at: 1, ...overrides };
}

function createService(overrides: Partial<DocsService> = {}): DocsService {
  return {
    getDocsSettings: vi.fn().mockResolvedValue({ root_dir: null, roots: [ROOT_A] }),
    upsertDocsSettings: vi.fn().mockImplementation((settings: { roots: DocsRoot[] }) => Promise.resolve(settings)),
    listDocsEntries: vi
      .fn()
      .mockResolvedValue([
        makeEntry({ name: '周报', path: '周报', is_dir: true }),
        makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false }),
      ]),
    readDocsFile: vi.fn().mockImplementation((_rootId: string, path: string) => Promise.resolve(`content of ${path}`)),
    saveDocsFile: vi.fn().mockResolvedValue(makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false })),
    createDocsFile: vi.fn().mockResolvedValue(makeEntry({ name: '新.md', path: '新.md', is_dir: false })),
    createDocsDirectory: vi.fn().mockResolvedValue(makeEntry({ name: '新目录', path: '新目录', is_dir: true })),
    renameDocsEntry: vi.fn().mockResolvedValue(makeEntry({ name: '改名.md', path: '改名.md', is_dir: false })),
    deleteDocsEntry: vi.fn().mockResolvedValue('removed'),
    searchDocs: vi.fn().mockResolvedValue([] as DocsSearchResult[]),
    ...overrides,
  };
}

/** 初始化列表后进入 ROOT_A，模拟用户从侧边栏点进一个文档库 */
async function bindStoreAtRoot(service: DocsService) {
  const store = bindStore(service);
  await store.initialize();
  await store.openRoot(ROOT_A.id);
  return store;
}

describe('docs store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initialize 只加载文档库列表，工作区按库加载条目', async () => {
    const service = createService();
    const store = bindStore(service);

    await store.initialize();

    expect(store.roots).toEqual([ROOT_A]);
    expect(store.activeRootId).toBe('');
    expect(service.listDocsEntries).not.toHaveBeenCalled();

    await store.openRoot(ROOT_A.id);

    expect(store.activeRootId).toBe('root-a');
    expect(store.activeRoot).toEqual(ROOT_A);
    expect(service.listDocsEntries).toHaveBeenCalledWith('root-a');
    expect(store.activePath).toBe('readme.md');
    expect(store.content).toBe('content of readme.md');
    expect(store.isDirty).toBe(false);
  });

  it('openRoot 脏且未确认时抛错，确认后放行并切库', async () => {
    const service = createService({
      listDocsEntries: vi
        .fn()
        .mockImplementation((rootId: string) =>
          Promise.resolve([makeEntry({ name: `${rootId}.md`, path: `${rootId}.md`, is_dir: false })]),
        ),
    });
    const store = await bindStoreAtRoot(service);

    store.content = '改过的内容';
    await expect(store.openRoot(ROOT_B.id)).rejects.toThrow('当前文档存在未保存修改');
    expect(store.activeRootId).toBe('root-a');

    await store.openRoot(ROOT_B.id, { discardUnsaved: true });

    expect(store.activeRootId).toBe('root-b');
    expect(store.activePath).toBe('root-b.md');
    expect(store.isDirty).toBe(false);
  });

  it('openRoot 同库重复进入是 no-op', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);

    await store.openRoot(ROOT_A.id);

    expect(store.activePath).toBe('readme.md');
    expect(service.listDocsEntries).toHaveBeenCalledTimes(1);
  });

  it('编辑产生脏状态，保存后回到干净', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);

    store.content = '改过的内容';
    expect(store.isDirty).toBe(true);

    expect(await store.save()).toBe(true);
    expect(service.saveDocsFile).toHaveBeenCalledWith('root-a', 'readme.md', '改过的内容');
    expect(store.isDirty).toBe(false);

    // 干净状态再保存是 no-op
    expect(await store.save()).toBe(false);
  });

  it('discardChanges 回滚到最后保存的内容', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);

    store.content = '临时乱写';
    store.discardChanges();
    expect(store.content).toBe('content of readme.md');
    expect(store.isDirty).toBe(false);
  });

  it('createFile 刷新列表并打开新文档', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);

    await store.createFile('周报', '2026-10-09');

    expect(service.createDocsFile).toHaveBeenCalledWith('root-a', '周报', '2026-10-09');
    expect(store.activePath).toBe('新.md');
  });

  it('重命名当前文档时同步活动路径', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);
    expect(store.activePath).toBe('readme.md');

    await store.renameEntry('readme.md', '新标题');

    expect(service.renameDocsEntry).toHaveBeenCalledWith('root-a', 'readme.md', '新标题');
    expect(store.activePath).toBe('改名.md');
    expect(store.content).toBe('content of readme.md');
  });

  it('删除当前文档后回退到第一篇', async () => {
    const deleteShown = { value: false };
    const service = createService({
      listDocsEntries: vi
        .fn()
        .mockImplementation(() =>
          Promise.resolve(
            deleteShown.value
              ? [
                  makeEntry({ name: '周报', path: '周报', is_dir: true }),
                  makeEntry({ name: 'a.md', path: 'a.md', is_dir: false }),
                ]
              : [
                  makeEntry({ name: '周报', path: '周报', is_dir: true }),
                  makeEntry({ name: 'a.md', path: 'a.md', is_dir: false }),
                  makeEntry({ name: 'b.md', path: 'b.md', is_dir: false }),
                ],
          ),
        ),
    });
    const store = bindStore(service);
    await store.initialize();
    await store.openRoot(ROOT_A.id);
    await store.openFile('b.md');

    deleteShown.value = true;
    await store.removeEntry('b.md');

    expect(store.activePath).toBe('a.md');
  });

  it('搜索带当前库 id，空关键词清空结果', async () => {
    const service = createService({
      searchDocs: vi
        .fn()
        .mockResolvedValue([{ path: 'a.md', name: 'a.md', line_number: 3, line_text: '包含 keyword 的行' }]),
    });
    const store = await bindStoreAtRoot(service);

    await store.runSearch('keyword');
    expect(service.searchDocs).toHaveBeenCalledWith('root-a', 'keyword');
    expect(store.searchResults).toHaveLength(1);

    await store.runSearch('   ');
    expect(store.searchResults).toHaveLength(0);
    expect(service.searchDocs).toHaveBeenCalledTimes(1);
  });

  it('saveRoots 保存新列表；移除当前库时清空工作区状态', async () => {
    const service = createService({
      upsertDocsSettings: vi.fn().mockImplementation((settings: { roots: DocsRoot[] }) => Promise.resolve(settings)),
    });
    const store = await bindStoreAtRoot(service);
    expect(store.activePath).toBe('readme.md');

    const saved = await store.saveRoots([ROOT_A, ROOT_B]);

    expect(service.upsertDocsSettings).toHaveBeenCalledWith({ root_dir: null, roots: [ROOT_A, ROOT_B] });
    expect(saved).toEqual([ROOT_A, ROOT_B]);
    expect(store.roots).toEqual([ROOT_A, ROOT_B]);
    expect(store.activePath).toBe('readme.md');

    await store.saveRoots([ROOT_B]);

    expect(store.roots).toEqual([ROOT_B]);
    expect(store.activeRootId).toBe('');
    expect(store.activePath).toBe('');
  });
});

describe('docs store 外部变更（MCP agent 写入）', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('本地干净时外部修改：防抖后静默跟进磁盘内容', async () => {
    const service = createService({
      readDocsFile: vi
        .fn()
        .mockImplementation((_rootId: string, path: string) =>
          Promise.resolve(path === 'readme.md' ? 'agent 写入的新内容' : `content of ${path}`),
        ),
    });
    const store = await bindStoreAtRoot(service);

    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'edit' });
    await vi.advanceTimersByTimeAsync(400);

    expect(store.content).toBe('agent 写入的新内容');
    expect(store.isDirty).toBe(false);
    expect(store.externalNotice).toBeNull();
  });

  it('本地有未保存修改时外部修改：提示且不覆盖，可选加载新版本或保留我的', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);
    store.content = '我的本地修改';

    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'write' });
    await vi.advanceTimersByTimeAsync(400);

    expect(store.content).toBe('我的本地修改');
    expect(store.externalNotice?.kind).toBe('modified');
    expect(store.externalNotice?.name).toBe('readme.md');
    vi.useRealTimers();

    // 加载新版本：放弃本地修改，跟进磁盘
    await store.acceptExternalVersion();
    expect(store.content).toBe('content of readme.md');
    expect(store.isDirty).toBe(false);
    expect(store.externalNotice).toBeNull();

    // 保留我的：只关提示，内容不动
    store.content = '再改一次';
    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'edit' });
    await vi.advanceTimersByTimeAsync(400);
    expect(store.externalNotice?.kind).toBe('modified');
    store.dismissExternalNotice();
    expect(store.externalNotice).toBeNull();
    expect(store.content).toBe('再改一次');
  });

  it('防抖窗口内的连续事件合并为一次处理', async () => {
    let disk = 'v1';
    const service = createService({
      readDocsFile: vi.fn().mockImplementation(() => Promise.resolve(disk)),
    });
    const store = await bindStoreAtRoot(service);
    const readsAfterSetup = vi.mocked(service.readDocsFile).mock.calls.length;

    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'write' });
    disk = 'v2';
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'edit' });
    await vi.advanceTimersByTimeAsync(400);

    expect(store.content).toBe('v2');
    expect(service.readDocsFile).toHaveBeenCalledTimes(readsAfterSetup + 1);
  });

  it('其他文档库的事件被忽略', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);
    const loadsAfterSetup = vi.mocked(service.listDocsEntries).mock.calls.length;

    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-b', path: 'readme.md', action: 'delete' });
    await vi.advanceTimersByTimeAsync(400);

    expect(service.listDocsEntries).toHaveBeenCalledTimes(loadsAfterSetup);
  });

  it('打开中的文档被外部删除：干净时回退第一篇，脏时提示删除', async () => {
    const service = createService({
      listDocsEntries: vi
        .fn()
        .mockResolvedValue([
          makeEntry({ name: 'a.md', path: 'a.md', is_dir: false }),
          makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false }),
        ]),
    });
    const store = await bindStoreAtRoot(service);
    expect(store.activePath).toBe('a.md');

    // 干净：回退到第一篇
    await store.openFile('readme.md');
    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'delete' });
    await vi.advanceTimersByTimeAsync(400);
    expect(store.activePath).toBe('a.md');
    expect(store.externalNotice).toBeNull();
    vi.useRealTimers();

    // 脏：内容保留，提示已删除（保存时会重新创建）
    await store.openFile('readme.md');
    store.content = '我的修改';
    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'delete' });
    await vi.advanceTimersByTimeAsync(400);
    expect(store.activePath).toBe('readme.md');
    expect(store.externalNotice?.kind).toBe('deleted');
    expect(store.externalNotice?.name).toBe('readme.md');
  });

  it('保存后外部修改提示失效（以本地版本为准）', async () => {
    const service = createService();
    const store = await bindStoreAtRoot(service);
    store.content = '我的本地修改';

    vi.useFakeTimers();
    store.handleExternalChange({ root_id: 'root-a', path: 'readme.md', action: 'edit' });
    await vi.advanceTimersByTimeAsync(400);
    expect(store.externalNotice?.kind).toBe('modified');
    vi.useRealTimers();

    await store.save();

    expect(store.externalNotice).toBeNull();
  });
});

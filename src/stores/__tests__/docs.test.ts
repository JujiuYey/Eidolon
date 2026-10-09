import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, defineStore, setActivePinia } from 'pinia';
import { createDocsStore } from '@/stores/docs';
import type { DocsService } from '@/stores/docs';
import type { DocsEntry, DocsSearchResult } from '@/types/docs';

function bindStore(service: DocsService) {
  const useBound = defineStore('docs-test', () => createDocsStore({ service }));
  return useBound();
}

function makeEntry(overrides: Partial<DocsEntry> & Pick<DocsEntry, 'name' | 'path' | 'is_dir'>): DocsEntry {
  return { size: 10, updated_at: 1, ...overrides };
}

function createService(overrides: Partial<DocsService> = {}): DocsService {
  return {
    getDocsSettings: vi.fn().mockResolvedValue({ root_dir: '/tmp/docs' }),
    upsertDocsSettings: vi.fn().mockResolvedValue({ root_dir: '/tmp/docs' }),
    listDocsEntries: vi.fn().mockResolvedValue([
      makeEntry({ name: '周报', path: '周报', is_dir: true }),
      makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false }),
    ]),
    readDocsFile: vi.fn().mockImplementation((path: string) => Promise.resolve(`content of ${path}`)),
    saveDocsFile: vi.fn().mockResolvedValue(makeEntry({ name: 'readme.md', path: 'readme.md', is_dir: false })),
    createDocsFile: vi.fn().mockResolvedValue(makeEntry({ name: '新.md', path: '新.md', is_dir: false })),
    createDocsDirectory: vi.fn().mockResolvedValue(makeEntry({ name: '新目录', path: '新目录', is_dir: true })),
    renameDocsEntry: vi.fn().mockResolvedValue(makeEntry({ name: '改名.md', path: '改名.md', is_dir: false })),
    deleteDocsEntry: vi.fn().mockResolvedValue('removed'),
    searchDocs: vi.fn().mockResolvedValue([] as DocsSearchResult[]),
    ...overrides,
  };
}

describe('docs store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initialize 默认打开第一篇文档', async () => {
    const service = createService();
    const store = bindStore(service);

    await store.initialize();

    expect(store.rootDir).toBe('/tmp/docs');
    expect(store.activePath).toBe('readme.md');
    expect(store.content).toBe('content of readme.md');
    expect(store.isDirty).toBe(false);
  });

  it('读取失败时切换到目标文件并保留错误', async () => {
    const service = createService({
      readDocsFile: vi.fn().mockRejectedValue('文件超过 2MB，暂不支持在应用内编辑'),
    });
    const store = bindStore(service);
    await store.initialize();

    await store.openFile('readme.md');

    expect(store.activePath).toBe('readme.md');
    expect(store.fileError).toBe('文件超过 2MB，暂不支持在应用内编辑');
  });

  it('编辑产生脏状态，保存后回到干净', async () => {
    const service = createService();
    const store = bindStore(service);
    await store.initialize();

    store.content = '改过的内容';
    expect(store.isDirty).toBe(true);

    expect(await store.save()).toBe(true);
    expect(service.saveDocsFile).toHaveBeenCalledWith('readme.md', '改过的内容');
    expect(store.isDirty).toBe(false);

    // 干净状态再保存是 no-op
    expect(await store.save()).toBe(false);
  });

  it('discardChanges 回滚到最后保存的内容', async () => {
    const service = createService();
    const store = bindStore(service);
    await store.initialize();

    store.content = '临时乱写';
    store.discardChanges();
    expect(store.content).toBe('content of readme.md');
    expect(store.isDirty).toBe(false);
  });

  it('createFile 刷新列表并打开新文档', async () => {
    const service = createService();
    const store = bindStore(service);
    await store.initialize();

    await store.createFile('周报', '2026-10-09');

    expect(service.createDocsFile).toHaveBeenCalledWith('周报', '2026-10-09');
    expect(store.activePath).toBe('新.md');
  });

  it('重命名当前文档时同步活动路径', async () => {
    const service = createService();
    const store = bindStore(service);
    await store.initialize();
    expect(store.activePath).toBe('readme.md');

    await store.renameEntry('readme.md', '新标题');

    expect(store.activePath).toBe('改名.md');
    expect(store.content).toBe('content of readme.md');
  });

  it('删除当前文档后回退到第一篇', async () => {
    const service = createService({
      listDocsEntries: vi.fn().mockResolvedValue([
        makeEntry({ name: '周报', path: '周报', is_dir: true }),
        makeEntry({ name: 'a.md', path: 'a.md', is_dir: false }),
        makeEntry({ name: 'b.md', path: 'b.md', is_dir: false }),
      ]),
    });
    const store = bindStore(service);
    await store.initialize();
    await store.openFile('b.md');

    service.listDocsEntries = vi.fn().mockResolvedValue([
      makeEntry({ name: '周报', path: '周报', is_dir: true }),
      makeEntry({ name: 'a.md', path: 'a.md', is_dir: false }),
    ]);
    await store.removeEntry('b.md');

    expect(store.activePath).toBe('a.md');
  });

  it('搜索空关键词清空结果', async () => {
    const service = createService({
      searchDocs: vi.fn().mockResolvedValue([
        { path: 'a.md', name: 'a.md', line_number: 3, line_text: '包含 keyword 的行' },
      ]),
    });
    const store = bindStore(service);

    await store.runSearch('keyword');
    expect(store.searchResults).toHaveLength(1);

    await store.runSearch('   ');
    expect(store.searchResults).toHaveLength(0);
    expect(service.searchDocs).toHaveBeenCalledTimes(1);
  });

  it('setRootDir 重置活动文档并重新加载', async () => {
    const service = createService({
      upsertDocsSettings: vi.fn().mockImplementation((settings: { root_dir: string | null }) =>
        Promise.resolve(settings)),
    });
    const store = bindStore(service);
    await store.initialize();
    expect(store.activePath).toBe('readme.md');

    await store.setRootDir('/new/vault');

    expect(service.upsertDocsSettings).toHaveBeenCalledWith({ root_dir: '/new/vault' });
    expect(store.rootDir).toBe('/new/vault');
    expect(store.activePath).toBe('readme.md');
  });
});

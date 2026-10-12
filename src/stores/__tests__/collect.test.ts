import { createPinia, defineStore, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createCollectStore } from '@/stores/collect';
import type { CollectService } from '@/stores/collect';
import type { CollectItem, LinkMetadata } from '@/types/collect';

function makeItem(overrides: Partial<CollectItem> = {}): CollectItem {
  return {
    id: 'collect_1',
    kind: 'link',
    url: 'https://example.com/post',
    title: null,
    description: null,
    content: '',
    favicon_url: null,
    archived: false,
    created_at: 100,
    updated_at: 100,
    deleted_at: null,
    ...overrides,
  };
}

function makeMetadata(overrides: Partial<LinkMetadata> = {}): LinkMetadata {
  return {
    title: '抓到的标题',
    description: '抓到的摘要',
    favicon_url: 'https://example.com/favicon.ico',
    ...overrides,
  };
}

function bindStore(service: CollectService) {
  const useBound = defineStore('collect-test', () => createCollectStore({ service }));
  return useBound();
}

function createService(overrides: Partial<CollectService> = {}): CollectService {
  return {
    listCollectItems: vi.fn().mockResolvedValue([]),
    listDeletedCollectItems: vi.fn().mockResolvedValue([]),
    restoreCollectItem: vi
      .fn()
      .mockImplementation(async (itemId: string) => makeItem({ id: itemId, deleted_at: null })),
    purgeCollectItem: vi.fn().mockResolvedValue('collect_1'),
    emptyCollectTrash: vi.fn().mockResolvedValue(0),
    upsertCollectItem: vi
      .fn()
      .mockImplementation(async (item: CollectItem) => ({ ...item, id: item.id || 'collect_new' })),
    deleteCollectItem: vi.fn().mockResolvedValue('collect_1'),
    fetchLinkMetadata: vi.fn().mockResolvedValue(makeMetadata()),
    summarizeCollectLink: vi.fn().mockResolvedValue('- 要点一\n- 要点二'),
    askCollectItems: vi.fn().mockResolvedValue('依据：正文提取入门 https://example.com/post'),
    openInBrowser: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

describe('collect store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loadItems fills the list and clears stale errors', async () => {
    const service = createService({ listCollectItems: vi.fn().mockResolvedValue([makeItem()]) });
    const store = bindStore(service);

    store.itemsError = '旧错误';
    await store.loadItems();

    expect(store.items).toHaveLength(1);
    expect(store.itemsLoaded).toBe(true);
    expect(store.itemsError).toBe('');
    expect(store.itemsLoading).toBe(false);
  });

  it('loadItems records errors without throwing', async () => {
    const service = createService({ listCollectItems: vi.fn().mockRejectedValue('数据库打不开') });
    const store = bindStore(service);

    await store.loadItems();

    expect(store.itemsError).toBe('数据库打不开');
    expect(store.itemsLoaded).toBe(false);
  });

  it('createFromText saves http(s) text as a link and enriches it in the background', async () => {
    const service = createService();
    const store = bindStore(service);

    const saved = await store.createFromText('  https://example.com/post  ');

    expect(service.upsertCollectItem).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'link', url: 'https://example.com/post', title: null }),
    );
    expect(saved.kind).toBe('link');
    expect(store.items).toHaveLength(1);

    // 后台抓取完成后回填元数据
    await vi.waitFor(() => {
      expect(store.items[0]?.title).toBe('抓到的标题');
    });
    expect(service.fetchLinkMetadata).toHaveBeenCalledWith('https://example.com/post');
    expect(store.items[0]?.description).toBe('抓到的摘要');
    expect(store.items[0]?.favicon_url).toBe('https://example.com/favicon.ico');
  });

  it('createFromText saves other text as a note without fetching metadata', async () => {
    const service = createService();
    const store = bindStore(service);

    await store.createFromText('  周五前交周报  ');

    expect(service.upsertCollectItem).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'note', content: '周五前交周报' }),
    );
    expect(service.fetchLinkMetadata).not.toHaveBeenCalled();
    expect(store.items[0]?.kind).toBe('note');
  });

  it('background enrichment failure keeps the placeholder and does not throw', async () => {
    const service = createService({ fetchLinkMetadata: vi.fn().mockRejectedValue('网络错误') });
    const store = bindStore(service);

    const saved = await store.createFromText('https://example.com/post');

    await vi.waitFor(() => {
      expect(service.fetchLinkMetadata).toHaveBeenCalled();
    });
    await new Promise(resolve => setTimeout(resolve, 0));

    expect(store.items[0]?.id).toBe(saved.id);
    expect(store.items[0]?.title).toBeNull();
    expect(store.items[0]?.url).toBe('https://example.com/post');
  });

  it('refreshLink overwrites the card with fresh metadata', async () => {
    const service = createService({
      listCollectItems: vi.fn().mockResolvedValue([makeItem({ title: '旧标题' })]),
      fetchLinkMetadata: vi.fn().mockResolvedValue(makeMetadata({ title: '新标题' })),
    });
    const store = bindStore(service);
    await store.loadItems();

    await store.refreshLink(store.items[0]!);

    expect(store.items[0]?.title).toBe('新标题');
    expect(store.items[0]?.description).toBe('抓到的摘要');
  });

  it('refreshLink ignores notes without url', async () => {
    const service = createService({
      listCollectItems: vi.fn().mockResolvedValue([makeItem({ kind: 'note', url: null })]),
    });
    const store = bindStore(service);
    await store.loadItems();

    await store.refreshLink(store.items[0]!);

    expect(service.fetchLinkMetadata).not.toHaveBeenCalled();
  });

  it('toggleArchived flips the flag and persists it', async () => {
    const service = createService({ listCollectItems: vi.fn().mockResolvedValue([makeItem()]) });
    const store = bindStore(service);
    await store.loadItems();

    const archived = await store.toggleArchived(store.items[0]!);
    expect(archived.archived).toBe(true);
    expect(store.items[0]?.archived).toBe(true);

    const restored = await store.toggleArchived(archived);
    expect(restored.archived).toBe(false);
  });

  it('removeItem calls the service and drops the item locally', async () => {
    const service = createService({ listCollectItems: vi.fn().mockResolvedValue([makeItem()]) });
    const store = bindStore(service);
    await store.loadItems();

    await store.removeItem('collect_1');

    expect(service.deleteCollectItem).toHaveBeenCalledWith('collect_1');
    expect(store.items).toHaveLength(0);
  });

  it('removeItem also drops the in-memory summary', async () => {
    const service = createService({ listCollectItems: vi.fn().mockResolvedValue([makeItem()]) });
    const store = bindStore(service);
    await store.loadItems();
    store.summaries['collect_1'] = { status: 'ready', text: '摘要', errorMessage: '' };

    await store.removeItem('collect_1');

    expect(store.summaries['collect_1']).toBeUndefined();
  });

  it('trash flow: loads deleted items, restores back to main list, purges and empties', async () => {
    const trashed = makeItem({ id: 'collect_trash', deleted_at: 12345 });
    const service = createService({
      listDeletedCollectItems: vi.fn().mockResolvedValue([trashed]),
      restoreCollectItem: vi.fn().mockResolvedValue(makeItem({ id: 'collect_trash', deleted_at: null })),
      emptyCollectTrash: vi.fn().mockResolvedValue(1),
    });
    const store = bindStore(service);

    await store.loadTrash();
    expect(store.trashItems).toHaveLength(1);
    expect(store.trashItems[0]?.id).toBe('collect_trash');

    // 恢复：移出回收站并回到主列表
    await store.restoreItem(store.trashItems[0]!);
    expect(service.restoreCollectItem).toHaveBeenCalledWith('collect_trash');
    expect(store.trashItems).toHaveLength(0);
    expect(store.items.map(item => item.id)).toContain('collect_trash');

    // 彻底删除单条
    await store.loadTrash();
    await store.purgeItem('collect_trash');
    expect(service.purgeCollectItem).toHaveBeenCalledWith('collect_trash');
    expect(store.trashItems).toHaveLength(0);

    // 清空回收站
    await store.loadTrash();
    const purged = await store.emptyTrash();
    expect(purged).toBe(1);
    expect(store.trashItems).toHaveLength(0);
  });

  it('summarizeLink loads then stores the summary', async () => {
    const service = createService({
      listCollectItems: vi.fn().mockResolvedValue([makeItem()]),
      summarizeCollectLink: vi.fn().mockResolvedValue('- 要点'),
    });
    const store = bindStore(service);
    await store.loadItems();

    const pending = store.summarizeLink(store.items[0]!);
    expect(store.summaries['collect_1']?.status).toBe('loading');
    await pending;

    expect(service.summarizeCollectLink).toHaveBeenCalledWith('https://example.com/post');
    expect(store.summaries['collect_1']).toEqual({ status: 'ready', text: '- 要点', errorMessage: '' });
  });

  it('summarizeLink keeps errors on state and ignores notes without url', async () => {
    const service = createService({
      listCollectItems: vi.fn().mockResolvedValue([makeItem()]),
      summarizeCollectLink: vi.fn().mockRejectedValue('请先配置收藏夹助手模型'),
    });
    const store = bindStore(service);
    await store.loadItems();

    // 记事没有 url：直接忽略
    await store.summarizeLink(makeItem({ kind: 'note', url: null }));
    expect(service.summarizeCollectLink).not.toHaveBeenCalled();

    await store.summarizeLink(store.items[0]!);

    expect(store.summaries['collect_1']?.status).toBe('error');
    expect(store.summaries['collect_1']?.errorMessage).toBe('请先配置收藏夹助手模型');
  });

  it('summarizeLink ignores duplicate requests while loading', async () => {
    let resolveFirst: ((text: string) => void) | undefined;
    const service = createService({
      listCollectItems: vi.fn().mockResolvedValue([makeItem()]),
      summarizeCollectLink: vi.fn().mockImplementationOnce(
        () =>
          new Promise<string>(resolve => {
            resolveFirst = resolve;
          }),
      ),
    });
    const store = bindStore(service);
    await store.loadItems();

    const first = store.summarizeLink(store.items[0]!);
    await store.summarizeLink(store.items[0]!);
    expect(service.summarizeCollectLink).toHaveBeenCalledTimes(1);

    resolveFirst?.('第一次的摘要');
    await first;
    expect(store.summaries['collect_1']?.text).toBe('第一次的摘要');
  });

  it('dismissSummary drops the in-memory summary', async () => {
    const service = createService({ listCollectItems: vi.fn().mockResolvedValue([makeItem()]) });
    const store = bindStore(service);
    await store.loadItems();
    await store.summarizeLink(store.items[0]!);
    expect(store.summaries['collect_1']).toBeDefined();

    store.dismissSummary('collect_1');

    expect(store.summaries['collect_1']).toBeUndefined();
  });

  it('askCollection stores the ready answer and dismissAnswer resets it', async () => {
    const service = createService({ askCollectItems: vi.fn().mockResolvedValue('依据：正文提取入门') });
    const store = bindStore(service);

    await store.askCollection(' 我收藏过什么？ ');

    expect(service.askCollectItems).toHaveBeenCalledWith('我收藏过什么？');
    expect(store.aiAnswer.status).toBe('ready');
    expect(store.aiAnswer.text).toBe('依据：正文提取入门');

    store.dismissAnswer();
    expect(store.aiAnswer.status).toBe('idle');
    expect(store.aiAnswer.question).toBe('');
  });

  it('askCollection refuses new questions while answering and keeps errors on state', async () => {
    let resolveFirst: ((text: string) => void) | undefined;
    const service = createService({
      askCollectItems: vi.fn().mockImplementationOnce(
        () =>
          new Promise<string>(resolve => {
            resolveFirst = resolve;
          }),
      ),
    });
    const store = bindStore(service);

    const first = store.askCollection('第一个问题');
    expect(store.aiAnswer.status).toBe('loading');

    await store.askCollection('第二个问题');
    expect(service.askCollectItems).toHaveBeenCalledTimes(1);

    resolveFirst?.('第一个回答');
    await first;
    expect(store.aiAnswer.question).toBe('第一个问题');

    // 换一个全新 pinia 再绑定，避免 defineStore 同 id 复用上面的 store 实例
    setActivePinia(createPinia());
    const failing = createService({ askCollectItems: vi.fn().mockRejectedValue('模型请求失败') });
    const failingStore = bindStore(failing);
    await failingStore.askCollection('随便问');
    expect(failingStore.aiAnswer.status).toBe('error');
    expect(failingStore.aiAnswer.errorMessage).toBe('模型请求失败');
  });
});

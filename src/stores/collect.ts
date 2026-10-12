import { defineStore } from 'pinia';
import { ref } from 'vue';

import * as collectServiceApi from '@/services/collect';
import type { CollectItem, LinkMetadata } from '@/types/collect';
import { isLinkText } from '@/views/collect/utils/display';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface CollectService {
  listCollectItems: () => Promise<CollectItem[]>;
  upsertCollectItem: (item: CollectItem) => Promise<CollectItem>;
  deleteCollectItem: (itemId: string) => Promise<string>;
  fetchLinkMetadata: (url: string) => Promise<LinkMetadata>;
  summarizeCollectLink: (url: string) => Promise<string>;
  askCollectItems: (question: string) => Promise<string>;
  openInBrowser: (url: string) => Promise<void>;
}

const defaultService: CollectService = collectServiceApi;

/** 单条链接的 AI 摘要状态（内存态，不落库） */
export interface AiSummaryState {
  status: 'loading' | 'ready' | 'error';
  text: string;
  errorMessage: string;
}

/** 「问收藏」问答状态 */
export interface AiAnswerState {
  status: 'idle' | 'loading' | 'ready' | 'error';
  question: string;
  text: string;
  errorMessage: string;
}

function emptyAnswer(): AiAnswerState {
  return { status: 'idle', question: '', text: '', errorMessage: '' };
}

function emptyItem(): CollectItem {
  return {
    id: '',
    kind: 'note',
    url: null,
    title: null,
    description: null,
    content: '',
    favicon_url: null,
    archived: false,
    created_at: 0,
    updated_at: 0,
  };
}

/**
 * 收藏夹 store（工厂模式，便于测试注入 fake service）。
 *
 * 核心是「秒存优先」：贴链接先落库（标题空着，卡片用域名占位），
 * 再后台抓取元数据回填；抓取失败静默，可从卡片菜单手动重试。
 */
export function createCollectStore(options: { service?: CollectService } = {}) {
  const service = options.service ?? defaultService;

  const items = ref<CollectItem[]>([]);
  const itemsLoading = ref(false);
  const itemsLoaded = ref(false);
  const itemsError = ref('');

  const summaries = ref<Record<string, AiSummaryState>>({});
  const aiAnswer = ref<AiAnswerState>(emptyAnswer());

  async function loadItems() {
    itemsLoading.value = true;
    try {
      items.value = await service.listCollectItems();
      itemsLoaded.value = true;
      itemsError.value = '';
    } catch (error) {
      itemsError.value = error instanceof Error ? error.message : String(error);
    } finally {
      itemsLoading.value = false;
    }
  }

  function upsertLocal(saved: CollectItem) {
    const index = items.value.findIndex(item => item.id === saved.id);
    if (index >= 0) {
      items.value.splice(index, 1, saved);
    } else {
      items.value.push(saved);
    }
  }

  /** 新建或更新条目；返回保存后的记录 */
  async function saveItem(item: CollectItem): Promise<CollectItem> {
    const saved = await service.upsertCollectItem(item);
    upsertLocal(saved);
    return saved;
  }

  /** 快速收集：http(s) 开头存链接卡，否则存记事卡 */
  async function createFromText(text: string): Promise<CollectItem> {
    const trimmed = text.trim();
    if (isLinkText(trimmed)) {
      const saved = await saveItem({ ...emptyItem(), kind: 'link', url: trimmed });
      // 秒存已返回，元数据抓取放后台；不阻塞输入框清空
      void enrichLink(saved.id, saved.url ?? '');
      return saved;
    }
    return saveItem({ ...emptyItem(), kind: 'note', content: trimmed });
  }

  /**
   * 抓取链接元数据并回填。失败静默（占位卡片保留，用户可手动重试）；
   * 自动回填只补空字段（避免覆盖用户先填的标题），手动重试则整卡刷新。
   */
  async function enrichLink(itemId: string, url: string, options: { overwrite?: boolean } = {}) {
    try {
      const metadata = await service.fetchLinkMetadata(url);
      const current = items.value.find(item => item.id === itemId);
      if (!current) {
        return; // 抓取期间条目已被删除
      }
      const pick = options.overwrite
        ? (fresh: string | null, existing: string | null) => fresh ?? existing
        : (fresh: string | null, existing: string | null) => existing ?? fresh;
      await saveItem({
        ...current,
        title: pick(metadata.title, current.title),
        description: pick(metadata.description, current.description),
        favicon_url: pick(metadata.favicon_url, current.favicon_url),
      });
    } catch {
      // 静默失败：链接本身已保存，占位卡片可从菜单「重新获取」
    }
  }

  /** 手动重新获取链接信息（整卡用最新抓取结果刷新） */
  async function refreshLink(item: CollectItem) {
    if (!item.url) {
      return;
    }
    await enrichLink(item.id, item.url, { overwrite: true });
  }

  /** 在系统默认浏览器中打开链接 */
  async function openInBrowser(url: string) {
    await service.openInBrowser(url);
  }

  async function toggleArchived(item: CollectItem): Promise<CollectItem> {
    return saveItem({ ...item, archived: !item.archived });
  }

  async function removeItem(itemId: string) {
    await service.deleteCollectItem(itemId);
    items.value = items.value.filter(item => item.id !== itemId);
    // 摘要是内存态，条目删了摘要也没意义
    dismissSummary(itemId);
  }

  // ===== AI：单条链接摘要（内存态，不落库） =====

  /** 生成链接摘要；同一链接生成中时忽略重复请求，已有结果再点是重新生成 */
  async function summarizeLink(item: CollectItem) {
    if (!item.url || summaries.value[item.id]?.status === 'loading') {
      return;
    }
    summaries.value[item.id] = { status: 'loading', text: '', errorMessage: '' };
    try {
      const text = await service.summarizeCollectLink(item.url);
      summaries.value[item.id] = { status: 'ready', text, errorMessage: '' };
    } catch (error) {
      summaries.value[item.id] = {
        status: 'error',
        text: '',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function dismissSummary(itemId: string) {
    delete summaries.value[itemId];
  }

  // ===== AI：问收藏（单轮问答） =====

  /** 基于全部收藏提问；回答中时拒绝新请求 */
  async function askCollection(question: string) {
    const text = question.trim();
    if (!text || aiAnswer.value.status === 'loading') {
      return;
    }
    aiAnswer.value = { status: 'loading', question: text, text: '', errorMessage: '' };
    try {
      const answer = await service.askCollectItems(text);
      aiAnswer.value = { status: 'ready', question: text, text: answer, errorMessage: '' };
    } catch (error) {
      aiAnswer.value = {
        status: 'error',
        question: text,
        text: '',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function dismissAnswer() {
    aiAnswer.value = emptyAnswer();
  }

  return {
    // 状态
    items,
    itemsLoading,
    itemsLoaded,
    itemsError,
    summaries,
    aiAnswer,
    // 动作
    loadItems,
    saveItem,
    createFromText,
    refreshLink,
    openInBrowser,
    toggleArchived,
    removeItem,
    summarizeLink,
    dismissSummary,
    askCollection,
    dismissAnswer,
  };
}

export type CollectStore = ReturnType<typeof createCollectStore>;

export const useCollectStore = defineStore('collect', () => createCollectStore());

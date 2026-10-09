import type { PmConversation, PmMessage } from '@/types/pm';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as pmServiceApi from '@/services/pm';
import { getErrorMessage } from '@/utils/helpers';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface PmService {
  listPmConversations: () => Promise<PmConversation[]>;
  createPmConversation: () => Promise<PmConversation>;
  renamePmConversation: (conversationId: string, title: string) => Promise<PmConversation>;
  deletePmConversation: (conversationId: string) => Promise<string>;
  listPmConversationMessages: (conversationId: string) => Promise<PmMessage[]>;
  sendPmMessage: (conversationId: string, content: string) => Promise<PmMessage>;
}

const defaultService: PmService = pmServiceApi;

function sortConversations(conversations: PmConversation[]): PmConversation[] {
  return [...conversations].sort(
    (left, right) =>
      right.updated_at - left.updated_at
      || left.title.localeCompare(right.title)
      || left.id.localeCompare(right.id),
  );
}

/**
 * 产品经理分身 store（工厂模式，便于测试注入 fake service）。
 *
 * 会话列表 + 当前会话消息 + 发送状态机。发送是乐观更新：先在界面上
 * 追加本地用户消息，后端成功落库后追加助手消息并刷新会话列表；
 * 失败时撤回本地消息并保留 sendError，输入草稿由视图自行保留。
 */
export function createPmStore(options: { service?: PmService } = {}) {
  const service = options.service ?? defaultService;

  const conversations = ref<PmConversation[]>([]);
  const conversationsLoading = ref(false);

  const activeConversationId = ref('');
  const messages = ref<PmMessage[]>([]);
  const messagesLoading = ref(false);

  const sending = ref(false);
  const sendError = ref('');

  /** 已完成加载的会话 id，避免重复拉取；递增 requestId 防止陈旧加载覆盖 */
  const loadedConversationId = ref('');
  const messagesRequestId = ref(0);
  let localMessageSeq = 0;

  const activeConversation = computed(
    () => conversations.value.find(conversation => conversation.id === activeConversationId.value) ?? null,
  );

  async function loadConversations() {
    conversationsLoading.value = true;
    try {
      conversations.value = sortConversations(await service.listPmConversations());
      // 当前会话已不存在（如刚被删除）时回退到最近一个
      if (!conversations.value.some(item => item.id === activeConversationId.value)) {
        const latest = conversations.value[0];
        if (latest) {
          await selectConversation(latest.id);
        } else {
          resetToEmpty();
        }
      }
    } finally {
      conversationsLoading.value = false;
    }
  }

  async function selectConversation(conversationId: string) {
    activeConversationId.value = conversationId;
    sendError.value = '';
    if (loadedConversationId.value === conversationId) {
      return;
    }

    const requestId = ++messagesRequestId.value;
    messages.value = [];

    messagesLoading.value = true;
    try {
      const loaded = await service.listPmConversationMessages(conversationId);
      if (messagesRequestId.value !== requestId) {
        return;
      }
      messages.value = loaded;
      loadedConversationId.value = conversationId;
    } finally {
      if (messagesRequestId.value === requestId) {
        messagesLoading.value = false;
      }
    }
  }

  async function createConversation(): Promise<PmConversation> {
    const created = await service.createPmConversation();
    conversations.value = sortConversations([created, ...conversations.value]);
    await selectConversation(created.id);
    return created;
  }

  async function renameConversation(conversationId: string, title: string) {
    const renamed = await service.renamePmConversation(conversationId, title);
    conversations.value = sortConversations(
      conversations.value.map(item => (item.id === renamed.id ? renamed : item)),
    );
  }

  async function removeConversation(conversationId: string) {
    await service.deletePmConversation(conversationId);
    conversations.value = conversations.value.filter(item => item.id !== conversationId);
    if (activeConversationId.value === conversationId) {
      const latest = conversations.value[0];
      if (latest) {
        await selectConversation(latest.id);
      } else {
        resetToEmpty();
      }
    }
  }

  /**
   * 发送一轮对话。没有活动会话时先自动创建。
   * 返回是否成功——成功后视图清空输入草稿，失败时草稿保留。
   */
  async function send(content: string): Promise<boolean> {
    const trimmed = content.trim();
    if (!trimmed || sending.value) {
      return false;
    }

    sending.value = true;
    sendError.value = '';

    try {
      const conversationId
        = activeConversationId.value || (await createConversation()).id;

      // 仅当仍停留在原会话时才展示乐观消息
      const showOptimistic = activeConversationId.value === conversationId;
      if (showOptimistic) {
        messages.value = [
          ...messages.value,
          {
            id: `local_${++localMessageSeq}`,
            conversation_id: conversationId,
            role: 'user',
            content: trimmed,
            skills_used: [],
            created_at: Date.now(),
          },
        ];
      }

      const assistant = await service.sendPmMessage(conversationId, trimmed);

      // 发送期间切换了会话：结果静默提交，不打扰当前展示
      if (activeConversationId.value === conversationId) {
        messages.value = [...messages.value, assistant];
      }
      await refreshConversationList(conversationId);
      return true;
    } catch (error) {
      sendError.value = getErrorMessage(error, '发送失败，请稍后重试');
      // 撤回乐观消息，草稿由视图恢复
      messages.value = messages.value.filter(message => !message.id.startsWith('local_'));
      return false;
    } finally {
      sending.value = false;
    }
  }

  async function refreshConversationList(activeConversationIdHint?: string) {
    try {
      const list = await service.listPmConversations();
      conversations.value = sortConversations(list);
      // 会话可能已被其他入口删除，兜底回最近一个
      const keepId = activeConversationIdHint ?? activeConversationId.value;
      if (!list.some(item => item.id === keepId)) {
        const latest = list[0];
        if (latest) {
          await selectConversation(latest.id);
        } else {
          resetToEmpty();
        }
      }
    } catch {
      // 列表刷新失败不阻断对话主流程
    }
  }

  function resetToEmpty() {
    activeConversationId.value = '';
    loadedConversationId.value = '';
    messages.value = [];
  }

  return {
    // 状态
    conversations,
    conversationsLoading,
    activeConversationId,
    messages,
    messagesLoading,
    sending,
    sendError,
    // 派生
    activeConversation,
    // 动作
    loadConversations,
    selectConversation,
    createConversation,
    renameConversation,
    removeConversation,
    send,
  };
}

export type PmStore = ReturnType<typeof createPmStore>;

export const usePmStore = defineStore('pm', () => createPmStore());

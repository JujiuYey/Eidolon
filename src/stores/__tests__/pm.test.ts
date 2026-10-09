import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, defineStore, setActivePinia } from 'pinia';
import { createPmStore } from '@/stores/pm';
import type { PmService } from '@/stores/pm';
import type { PmConversation, PmMessage } from '@/types/pm';

function bindStore(service: PmService) {
  const useBound = defineStore('pm-test', () => createPmStore({ service }));
  return useBound();
}

function makeConversation(overrides: Partial<PmConversation> = {}): PmConversation {
  return {
    id: 'conv_1',
    title: '新对话',
    created_at: 1,
    updated_at: 1,
    ...overrides,
  };
}

function makeMessage(overrides: Partial<PmMessage> = {}): PmMessage {
  return {
    id: 'msg_1',
    conversation_id: 'conv_1',
    role: 'user',
    content: '你好',
    skills_used: [],
    created_at: 1,
    ...overrides,
  };
}

function createService(overrides: Partial<PmService> = {}): PmService {
  return {
    listPmConversations: vi.fn().mockResolvedValue([]),
    createPmConversation: vi.fn().mockResolvedValue(makeConversation()),
    renamePmConversation: vi.fn().mockResolvedValue(makeConversation()),
    deletePmConversation: vi.fn().mockResolvedValue('conv_1'),
    listPmConversationMessages: vi.fn().mockResolvedValue([]),
    sendPmMessage: vi.fn().mockResolvedValue(makeMessage({ role: 'assistant' })),
    ...overrides,
  };
}

describe('pm store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loadConversations 自动选中最近会话并加载消息', async () => {
    const older = makeConversation({ id: 'conv_a', updated_at: 100 });
    const newer = makeConversation({ id: 'conv_b', updated_at: 200, title: '评审' });
    const service = createService({
      listPmConversations: vi.fn().mockResolvedValue([older, newer]),
      listPmConversationMessages: vi.fn().mockImplementation((id: string) =>
        Promise.resolve(id === 'conv_b' ? [makeMessage({ conversation_id: 'conv_b' })] : [])),
    });
    const store = bindStore(service);

    await store.loadConversations();

    expect(store.activeConversationId).toBe('conv_b');
    expect(store.messages).toHaveLength(1);
    expect(store.messages[0]?.conversation_id).toBe('conv_b');
  });

  it('列表为空时清空活动会话', async () => {
    const service = createService();
    const store = bindStore(service);
    store.activeConversationId = 'conv_gone';

    await store.loadConversations();

    expect(store.activeConversationId).toBe('');
    expect(store.messages).toHaveLength(0);
  });

  it('selectConversation 重复选择同一会话不重复拉取', async () => {
    const listMessages = vi.fn().mockResolvedValue([makeMessage()]);
    const service = createService({ listPmConversationMessages: listMessages });
    const store = bindStore(service);

    await store.selectConversation('conv_1');
    await store.selectConversation('conv_1');

    expect(listMessages).toHaveBeenCalledTimes(1);
  });

  it('send 无活动会话时自动创建并追加乐观消息与助手回复', async () => {
    const created = makeConversation({ id: 'conv_new', updated_at: 500 });
    const assistant = makeMessage({
      id: 'msg_reply',
      conversation_id: 'conv_new',
      role: 'assistant',
      content: '好的，我们先明确目标用户。',
      skills_used: ['double-diamond'],
      created_at: 501,
    });
    const service = createService({
      createPmConversation: vi.fn().mockResolvedValue(created),
      sendPmMessage: vi.fn().mockResolvedValue(assistant),
      listPmConversations: vi.fn().mockResolvedValue([created]),
    });
    const store = bindStore(service);

    const ok = await store.send('帮我梳理一个需求');

    expect(ok).toBe(true);
    expect(store.activeConversationId).toBe('conv_new');
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0]?.role).toBe('user');
    expect(store.messages[0]?.content).toBe('帮我梳理一个需求');
    expect(store.messages[1]?.skills_used).toEqual(['double-diamond']);
    expect(store.sending).toBe(false);
    expect(store.sendError).toBe('');
    expect(service.sendPmMessage).toHaveBeenCalledWith('conv_new', '帮我梳理一个需求');
  });

  it('send 失败时撤回乐观消息并保留错误', async () => {
    const service = createService({
      sendPmMessage: vi.fn().mockRejectedValue('模型请求失败'),
    });
    const store = bindStore(service);
    await store.selectConversation('conv_1');

    const ok = await store.send('你好');

    expect(ok).toBe(false);
    expect(store.messages).toHaveLength(0);
    expect(store.sendError).toBe('模型请求失败');
    expect(store.sending).toBe(false);
  });

  it('空内容或发送中不重复发送', async () => {
    const service = createService();
    const store = bindStore(service);

    expect(await store.send('   ')).toBe(false);
    expect(service.sendPmMessage).not.toHaveBeenCalled();

    store.sending = true;
    expect(await store.send('你好')).toBe(false);
    expect(service.sendPmMessage).not.toHaveBeenCalled();
  });

  it('发送期间切换会话时结果静默提交', async () => {
    const convA = makeConversation({ id: 'conv_a', updated_at: 100 });
    const convB = makeConversation({ id: 'conv_b', updated_at: 200 });
    let resolveSend: (message: PmMessage) => void = () => {};
    const sendPromise = new Promise<PmMessage>(resolve => {
      resolveSend = resolve;
    });
    const service = createService({
      listPmConversations: vi.fn().mockResolvedValue([convA, convB]),
      listPmConversationMessages: vi.fn().mockImplementation((id: string) =>
        Promise.resolve(id === 'conv_b' ? [makeMessage({ id: 'msg_b', conversation_id: 'conv_b' })] : [])),
      sendPmMessage: vi.fn().mockImplementation(() => sendPromise),
    });
    const store = bindStore(service);
    await store.selectConversation('conv_a');

    const pending = store.send('在会话 A 提问');
    await Promise.resolve();
    await store.selectConversation('conv_b');

    resolveSend(makeMessage({ id: 'msg_reply', conversation_id: 'conv_a', role: 'assistant' }));
    expect(await pending).toBe(true);

    // 仍停留在会话 B：既没有乐观用户消息，也没有 A 的回复
    expect(store.messages).toHaveLength(1);
    expect(store.messages[0]?.id).toBe('msg_b');
  });

  it('renameConversation 更新列表条目', async () => {
    const service = createService({
      renamePmConversation: vi.fn().mockResolvedValue(
        makeConversation({ id: 'conv_1', title: '登录流程评审', updated_at: 300 }),
      ),
    });
    const store = bindStore(service);
    store.conversations = [makeConversation()];

    await store.renameConversation('conv_1', '登录流程评审');

    expect(store.conversations[0]?.title).toBe('登录流程评审');
  });

  it('removeConversation 删除活动会话后回退到最近会话', async () => {
    const convA = makeConversation({ id: 'conv_a', updated_at: 100 });
    const convB = makeConversation({ id: 'conv_b', updated_at: 200 });
    const service = createService({
      listPmConversations: vi.fn().mockResolvedValue([convA, convB]),
      listPmConversationMessages: vi.fn().mockResolvedValue([]),
    });
    const store = bindStore(service);
    await store.loadConversations();
    expect(store.activeConversationId).toBe('conv_b');

    await store.removeConversation('conv_b');

    expect(store.conversations.map(item => item.id)).toEqual(['conv_a']);
    expect(store.activeConversationId).toBe('conv_a');
  });
});

import type {
  ZentaoAccount,
  ZentaoMyWork,
} from '@/types/zentao';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as zentaoServiceApi from '@/services/zentao';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface ZentaoService {
  listZentaoAccounts: () => Promise<ZentaoAccount[]>;
  upsertZentaoAccount: (account: ZentaoAccount) => Promise<ZentaoAccount>;
  deleteZentaoAccount: (accountId: string) => Promise<string>;
  fetchZentaoMyWork: (accountId: string) => Promise<ZentaoMyWork>;
}

const defaultService: ZentaoService = zentaoServiceApi;

/**
 * 禅道 store（工厂模式，便于测试注入 fake service）。
 *
 * 维护账户列表与当前选中账户的"我的工作台"数据；
 * 切换账户时清空旧数据，由视图触发刷新。
 */
export function createZentaoStore(options: { service?: ZentaoService } = {}) {
  const service = options.service ?? defaultService;

  const accounts = ref<ZentaoAccount[]>([]);
  const accountsLoading = ref(false);
  const accountsLoaded = ref(false);

  const activeAccountId = ref('');
  const myWork = ref<ZentaoMyWork | null>(null);
  const workLoading = ref(false);
  const workError = ref('');

  const enabledAccounts = computed(() =>
    accounts.value.filter(account => account.enabled),
  );

  const activeAccount = computed(
    () => accounts.value.find(account => account.id === activeAccountId.value) ?? null,
  );

  async function loadAccounts() {
    accountsLoading.value = true;
    try {
      accounts.value = await service.listZentaoAccounts();
      accountsLoaded.value = true;
      // 当前账户不存在时回退到第一个启用的账户
      if (!accounts.value.some(account => account.id === activeAccountId.value)) {
        activeAccountId.value = enabledAccounts.value[0]?.id ?? '';
        myWork.value = null;
        workError.value = '';
      }
    } finally {
      accountsLoading.value = false;
    }
  }

  /** 新建或更新账户；返回保存后的记录 */
  async function saveAccount(account: ZentaoAccount): Promise<ZentaoAccount> {
    const saved = await service.upsertZentaoAccount(account);
    const index = accounts.value.findIndex(item => item.id === saved.id);
    if (index >= 0) {
      accounts.value.splice(index, 1, saved);
    } else {
      accounts.value.push(saved);
      accounts.value.sort((a, b) => a.sort - b.sort || a.created_at - b.created_at);
    }
    if (!activeAccountId.value) {
      activeAccountId.value = saved.id;
    }
    return saved;
  }

  async function removeAccount(accountId: string) {
    await service.deleteZentaoAccount(accountId);
    accounts.value = accounts.value.filter(account => account.id !== accountId);
    if (activeAccountId.value === accountId) {
      activeAccountId.value = enabledAccounts.value[0]?.id ?? '';
      myWork.value = null;
      workError.value = '';
    }
  }

  function selectAccount(accountId: string) {
    if (activeAccountId.value === accountId) {
      return;
    }
    activeAccountId.value = accountId;
    myWork.value = null;
    workError.value = '';
  }

  async function refreshWork() {
    const accountId = activeAccountId.value;
    if (!accountId || workLoading.value) {
      return;
    }
    workLoading.value = true;
    workError.value = '';
    try {
      myWork.value = await service.fetchZentaoMyWork(accountId);
    } catch (error) {
      myWork.value = null;
      workError.value = error instanceof Error ? error.message : String(error);
    } finally {
      workLoading.value = false;
    }
  }

  return {
    // 状态
    accounts,
    accountsLoading,
    accountsLoaded,
    activeAccountId,
    myWork,
    workLoading,
    workError,
    // 派生
    enabledAccounts,
    activeAccount,
    // 动作
    loadAccounts,
    saveAccount,
    removeAccount,
    selectAccount,
    refreshWork,
  };
}

export type ZentaoStore = ReturnType<typeof createZentaoStore>;

export const useZentaoStore = defineStore('zentao', () => createZentaoStore());

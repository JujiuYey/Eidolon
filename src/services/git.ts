import { invoke } from '@tauri-apps/api/core';

import type { GitAccount } from '@/types/git';

/** Git 账户 CRUD。仅设置页使用；周报后端直接读库，不经此处 */
export function listGitAccounts(): Promise<GitAccount[]> {
  return invoke<GitAccount[]>('list_git_accounts');
}

export function upsertGitAccount(account: GitAccount): Promise<GitAccount> {
  return invoke<GitAccount>('upsert_git_account', { account });
}

export function deleteGitAccount(accountId: string): Promise<string> {
  return invoke<string>('delete_git_account', { accountId });
}

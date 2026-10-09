import type {
  TestZentaoConnectionRequest,
  TestZentaoConnectionResponse,
  ZentaoAccount,
  ZentaoMyWork,
} from '@/types/zentao';
import { invoke } from '@tauri-apps/api/core';

export async function listZentaoAccounts(): Promise<ZentaoAccount[]> {
  return invoke<ZentaoAccount[]>('list_zentao_accounts');
}

export async function upsertZentaoAccount(account: ZentaoAccount): Promise<ZentaoAccount> {
  return invoke<ZentaoAccount>('upsert_zentao_account', { account });
}

export async function deleteZentaoAccount(accountId: string): Promise<string> {
  return invoke<string>('delete_zentao_account', { accountId });
}

export async function testZentaoConnection(
  request: TestZentaoConnectionRequest,
): Promise<TestZentaoConnectionResponse> {
  return invoke<TestZentaoConnectionResponse>('test_zentao_connection', { request });
}

export async function fetchZentaoMyWork(accountId: string): Promise<ZentaoMyWork> {
  return invoke<ZentaoMyWork>('fetch_zentao_my_work', { accountId });
}

import { invoke } from '@tauri-apps/api/core';
import type { ApiAiGenerateInput, ApiAiGenerateResult } from '@/types/api-client';
import { toFrontendAiResult, toTauriAiInput } from './mappers';
import type { TauriAiGenerateResult } from './types';

export async function generateApiRequestBody(input: ApiAiGenerateInput): Promise<ApiAiGenerateResult> {
  const raw = await invoke<TauriAiGenerateResult>('generate_api_request_body', { input: toTauriAiInput(input) });
  return toFrontendAiResult(raw);
}

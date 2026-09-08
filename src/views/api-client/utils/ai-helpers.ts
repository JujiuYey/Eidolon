import type { ApiClientAiCandidate, ApiClientHttpMethod, ApiClientRequestBody } from '@/types/api-client';
import { tryFormatJson } from '@/views/api-client/utils/request-helpers';

export interface BuildAiCandidateOptions {
  requestId: string;
  taskId: string;
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  modelLabel: string;
  rawContent: string;
}

export function buildAiCandidate(options: BuildAiCandidateOptions): ApiClientAiCandidate {
  const trimmed = options.rawContent.trim();
  const jsonCheck = tryFormatJson(trimmed);

  return {
    requestId: options.requestId,
    taskId: options.taskId,
    prompt: options.prompt,
    reference: options.reference,
    includeCurrentBody: options.includeCurrentBody,
    content: options.rawContent,
    isJsonValid: jsonCheck.ok,
    jsonError: jsonCheck.error,
    status: 'success',
    errorMessage: null,
    modelLabel: options.modelLabel,
  };
}

export function buildAiCandidateGenerating(options: Omit<BuildAiCandidateOptions, 'rawContent'>): ApiClientAiCandidate {
  return {
    requestId: options.requestId,
    taskId: options.taskId,
    prompt: options.prompt,
    reference: options.reference,
    includeCurrentBody: options.includeCurrentBody,
    content: '',
    isJsonValid: false,
    jsonError: null,
    status: 'generating',
    errorMessage: null,
    modelLabel: options.modelLabel,
  };
}

export function buildAiCandidateFailed(options: {
  requestId: string;
  taskId: string;
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  modelLabel: string;
  errorMessage: string;
}): ApiClientAiCandidate {
  return {
    requestId: options.requestId,
    taskId: options.taskId,
    prompt: options.prompt,
    reference: options.reference,
    includeCurrentBody: options.includeCurrentBody,
    content: '',
    isJsonValid: false,
    jsonError: null,
    status: 'error',
    errorMessage: options.errorMessage,
    modelLabel: options.modelLabel,
  };
}

export function applyCandidateToBody(body: ApiClientRequestBody, candidate: ApiClientAiCandidate): ApiClientRequestBody {
  if (!candidate.isJsonValid) {
    return body;
  }
  if (body.kind !== 'json') {
    return {
      kind: 'json',
      text: candidate.content,
      form: [],
    };
  }
  return {
    kind: 'json',
    text: candidate.content,
    form: [],
  };
}

export interface AiContextBuildInput {
  prompt: string;
  reference: string;
  includeCurrentBody: boolean;
  currentBody: ApiClientRequestBody;
  requestName: string;
  method: ApiClientHttpMethod;
  url: string;
}

export interface AiContextPayload {
  prompt: string;
  reference: string;
  hasCurrentBody: boolean;
  currentBodyJson: string | null;
  currentBodyKind: ApiClientRequestBody['kind'];
  requestName: string;
  method: ApiClientHttpMethod;
  url: string;
}

export function buildAiContextPayload(input: AiContextBuildInput): AiContextPayload {
  return {
    prompt: input.prompt,
    reference: input.reference,
    hasCurrentBody: input.includeCurrentBody && input.currentBody.kind === 'json' && input.currentBody.text.trim().length > 0,
    currentBodyJson: input.includeCurrentBody && input.currentBody.kind === 'json'
      ? input.currentBody.text
      : null,
    currentBodyKind: input.currentBody.kind,
    requestName: input.requestName,
    method: input.method,
    url: input.url,
  };
}

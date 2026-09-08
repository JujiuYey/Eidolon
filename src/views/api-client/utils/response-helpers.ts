import type {
  ApiClientHistoryExecutionStatus,
  ApiClientResponseKind,
  ApiClientResponseView,
} from '@/types/api-client';

export interface ResponseBuildInput {
  kind: ApiClientResponseKind;
  rawText: string;
  meta: ApiClientResponseView['meta'];
  errorMessage: string | null;
  isOversized?: boolean;
  isBinary?: boolean;
  isJsonTruncated?: boolean;
}

export function buildResponseView(input: ResponseBuildInput): ApiClientResponseView {
  const formatted = input.kind === 'success' || input.kind === 'http_error'
    ? tryFormatResponseJson(input.rawText)
    : { formatted: null as string | null, isTruncated: !!input.isJsonTruncated };

  return {
    kind: input.kind,
    rawText: input.rawText,
    formattedJson: formatted.formatted,
    isJsonTruncated: formatted.isTruncated || !!input.isJsonTruncated,
    isOversized: !!input.isOversized,
    isBinary: !!input.isBinary,
    meta: input.meta,
    errorMessage: input.errorMessage,
  };
}

export function tryFormatResponseJson(rawText: string): { formatted: string | null; isTruncated: boolean } {
  if (!rawText.trim()) {
    return { formatted: null, isTruncated: false };
  }

  try {
    const parsed = JSON.parse(rawText);
    return { formatted: JSON.stringify(parsed, null, 2), isTruncated: false };
  } catch {
    return { formatted: null, isTruncated: false };
  }
}

export function classifyExecutionStatus(input: {
  ok: boolean;
  statusCode: number | null;
  cancelled: boolean;
  timedOut: boolean;
  connectionFailed: boolean;
}): ApiClientHistoryExecutionStatus {
  if (input.cancelled) {
    return 'cancelled';
  }
  if (input.timedOut) {
    return 'timeout';
  }
  if (input.connectionFailed) {
    return 'network_error';
  }
  if (input.ok && input.statusCode !== null && input.statusCode >= 200 && input.statusCode < 400) {
    return 'success';
  }
  return 'http_error';
}

export function classifyResponseKind(input: {
  cancelled: boolean;
  timedOut: boolean;
  connectionFailed: boolean;
  statusCode: number | null;
  isOversized: boolean;
  isBinary: boolean;
}): ApiClientResponseKind {
  if (input.cancelled) {
    return 'cancelled';
  }
  if (input.timedOut) {
    return 'timeout';
  }
  if (input.connectionFailed) {
    return 'network_error';
  }
  if (input.isOversized) {
    return 'oversize';
  }
  if (input.isBinary) {
    return 'binary';
  }
  if (input.statusCode === null) {
    return 'idle';
  }
  if (input.statusCode >= 200 && input.statusCode < 400) {
    return 'success';
  }
  return 'http_error';
}

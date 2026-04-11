export interface SagMarkdownSnippet {
  label: string;
  value: string;
}

export interface SagMarkdownGuide {
  description?: string;
  emptyPreviewContent?: string;
  primarySnippet?: SagMarkdownSnippet | null;
}

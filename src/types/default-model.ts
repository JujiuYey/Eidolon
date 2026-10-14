export type DefaultModelKey =
  | 'assistant'
  | 'quick'
  | 'translation'
  | 'embedding'
  | 'weekly_report_polish'
  | 'pm_chat'
  | 'docs_chat'
  | 'task_assistant'
  | 'collect_assistant';

export interface DefaultModelSetting {
  key: DefaultModelKey;
  provider_id: string;
  model_id: string;
  temperature: string;
  top_p: string;
  max_tokens: string;
  presence_penalty: string;
  frequency_penalty: string;
}

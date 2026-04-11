export interface AgentSettings {
  maxHistoryMessages: number;
  maxToolRounds: number;
  fileExtensions: string;
  ignoreDirs: string;
  maxFileContentLength: number;
}

export interface AgentToolTrace {
  id: string;
  name: string;
  argsText: string;
  resultText?: string;
  status: 'pending' | 'running' | 'success' | 'error';
}

export interface AgentPlanStep {
  id: string;
  title: string;
  status: 'pending' | 'running' | 'completed' | 'error';
  expectedOutput: string;
  recommendedTools: string[];
  note?: string | null;
  evidenceFiles: string[];
}

export interface AgentExecutionPlan {
  goal: string;
  summary: string;
  steps: AgentPlanStep[];
}

export interface AnalyzedModule {
  path: string;
  scopeType: 'file' | 'directory' | 'multi_file' | string;
  relatedPaths: string[];
  summary: string;
  updatedAt: string;
}

export interface AgentMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  createdAt: number;
  status?: 'streaming' | 'done' | 'error';
  toolTraces?: AgentToolTrace[];
}

export interface AgentConversation {
  id: string;
  title: string;
  messages: AgentMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface FileEntry {
  name: string;
  path: string;
  file_type: string;
}

export interface LineMatch {
  line_number: number;
  content: string;
  is_target: boolean;
}

export interface SearchResult {
  file: string;
  matches: LineMatch[];
}

export interface AgentMessageDto {
  id: string;
  role: string;
  content: string;
  created_at: number;
}

export interface AgentConversationDto {
  id: string;
  title: string;
  messages: AgentMessageDto[];
  created_at: number;
  updated_at: number;
}

export interface ModuleSummaryDto {
  path: string;
  scope_type?: 'file' | 'directory' | 'multi_file' | string;
  related_paths?: string[];
  summary: string;
  updated_at: string;
}

export interface AgentMemoryDataDto {
  conversations: AgentConversationDto[];
  analyzed_modules: Record<string, ModuleSummaryDto>;
}

export interface StartAgentRunResponse {
  run_id: string;
  conversation_id: string;
}

export interface AgentEventPayload<T = Record<string, unknown>> {
  run_id: string;
  conversation_id: string;
  event_type: string;
  data: T;
}

export interface RunStartedEventData {
  title?: string;
  profileId?: string;
  profileName?: string;
  planningEnabled?: boolean;
}

export interface AssistantDeltaEventData {
  delta?: string;
  content?: string;
}

export interface PlanCreatedEventData {
  plan: AgentExecutionPlan;
}

export interface StepStartedEventData {
  step: AgentPlanStep;
}

export interface StepFinishedEventData {
  step: AgentPlanStep;
}

export interface ToolStartedEventData {
  trace_id: string;
  name: string;
  args_text: string;
}

export interface ToolFinishedEventData extends ToolStartedEventData {
  result_text?: string;
  status?: 'success' | 'error';
}

export interface AnalysisSavedEventData {
  path: string;
  scope_type?: 'file' | 'directory' | 'multi_file' | string;
  related_paths?: string[];
  summary: string;
  updated_at: string;
}

export interface RunFinishedEventData {
  title?: string;
  final_message?: string;
}

export interface RunErrorEventData {
  message?: string;
}

import { invoke } from '@tauri-apps/api/core';

import type { Task, TaskDraft } from '@/types/tasks';

export async function listTasks(): Promise<Task[]> {
  return invoke<Task[]>('list_tasks');
}

export async function upsertTask(task: Task): Promise<Task> {
  return invoke<Task>('upsert_task', { task });
}

export async function deleteTask(taskId: string): Promise<string> {
  return invoke<string>('delete_task', { taskId });
}

/** AI：把一句话整理成结构化任务草稿 */
export async function parseTaskText(input: string): Promise<TaskDraft> {
  return invoke<TaskDraft>('parse_task_text', { input });
}

/** AI：根据未完成任务给出今日建议 */
export async function suggestTaskPlan(tasks: Task[]): Promise<string> {
  return invoke<string>('suggest_task_plan', { tasks });
}

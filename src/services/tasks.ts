import { invoke } from '@tauri-apps/api/core';

import type { Task, TaskDraft, TaskPlan } from '@/types/tasks';

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

/** AI：把未排期任务排成结构化时间块（前端过目后应用） */
export async function suggestTaskPlan(tasks: Task[]): Promise<TaskPlan> {
  return invoke<TaskPlan>('suggest_task_plan', { tasks });
}

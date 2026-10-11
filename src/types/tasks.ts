export type TaskStatus = 'todo' | 'doing' | 'done';

export type TaskPriority = 'high' | 'medium' | 'low' | 'none';

export interface Task {
  id: string;
  title: string;
  notes: string;
  status: TaskStatus;
  priority: TaskPriority;
  /** 截止日期 YYYY-MM-DD；空字符串表示无截止 */
  due_date: string;
  /** 来源：manual（预留外部来源） */
  source: string;
  sort: number;
  /** 完成时间 Unix 毫秒；未完成为 null */
  completed_at: number | null;
  created_at: number;
  updated_at: number;
}

/** AI 从一句话解析出的任务草稿 */
export interface TaskDraft {
  title: string;
  priority: TaskPriority;
  due_date: string;
}

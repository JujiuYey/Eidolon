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
  /** 排期开始，本地无时区 ISO（YYYY-MM-DDTHH:MM）；空字符串表示未排期 */
  start_at: string;
  /** 排期结束，本地无时区 ISO；与 start_at 成对出现 */
  end_at: string;
  /** 预估时长（分钟），拖入日历的默认块大小；0 = 未设置 */
  estimate_minutes: number;
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

/** AI 排期建议中的单个时间块（标题/优先级取自任务本体） */
export interface TaskPlanBlock {
  task_id: string;
  title: string;
  priority: TaskPriority;
  start_at: string;
  end_at: string;
}

/** AI 排期建议：结构化时间块 + 一句安排思路 */
export interface TaskPlan {
  blocks: TaskPlanBlock[];
  note: string;
}

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import * as tasksServiceApi from '@/services/tasks';
import type { Task, TaskDraft } from '@/types/tasks';

/** store 依赖的服务接口，测试时可注入 fake 实现 */
export interface TasksService {
  listTasks: () => Promise<Task[]>;
  upsertTask: (task: Task) => Promise<Task>;
  deleteTask: (taskId: string) => Promise<string>;
  parseTaskText: (input: string) => Promise<TaskDraft>;
  suggestTaskPlan: (tasks: Task[]) => Promise<string>;
}

const defaultService: TasksService = tasksServiceApi;

/** AI 草稿候选：taskId 用来丢弃过期结果，draft 可被用户改过后再落库 */
export interface AiDraftCandidate {
  taskId: string;
  input: string;
  status: 'generating' | 'success' | 'error';
  draft: TaskDraft;
  errorMessage: string;
}

export interface AiPlanState {
  status: 'loading' | 'ready' | 'error';
  text: string;
  errorMessage: string;
}

function emptyDraft(input: string): TaskDraft {
  return { title: input, priority: 'none', due_date: '' };
}

function emptyTask(): Task {
  return {
    id: '',
    title: '',
    notes: '',
    status: 'todo',
    priority: 'none',
    due_date: '',
    source: 'manual',
    sort: 0,
    completed_at: null,
    created_at: 0,
    updated_at: 0,
  };
}

/**
 * 个人任务 store（工厂模式，便于测试注入 fake service）。
 *
 * 维护任务清单与两个 AI 状态机：一句话整理草稿（候选模式）、今日建议（一次性）。
 */
export function createTasksStore(options: { service?: TasksService } = {}) {
  const service = options.service ?? defaultService;

  const tasks = ref<Task[]>([]);
  const tasksLoading = ref(false);
  const tasksLoaded = ref(false);
  const tasksError = ref('');

  const aiDraft = ref<AiDraftCandidate | null>(null);
  const aiPlan = ref<AiPlanState | null>(null);

  const openTasks = computed(() => tasks.value.filter(task => task.status !== 'done'));

  async function loadTasks() {
    tasksLoading.value = true;
    try {
      tasks.value = await service.listTasks();
      tasksLoaded.value = true;
      tasksError.value = '';
    } catch (error) {
      tasksError.value = error instanceof Error ? error.message : String(error);
    } finally {
      tasksLoading.value = false;
    }
  }

  function upsertLocal(saved: Task) {
    const index = tasks.value.findIndex(item => item.id === saved.id);
    if (index >= 0) {
      tasks.value.splice(index, 1, saved);
    } else {
      tasks.value.push(saved);
    }
  }

  /** 新建或更新任务；返回保存后的记录 */
  async function saveTask(task: Task): Promise<Task> {
    const saved = await service.upsertTask(task);
    upsertLocal(saved);
    return saved;
  }

  /** 快速添加：原文即标题（不依赖 AI，永远可用） */
  async function createFromText(text: string): Promise<Task> {
    return saveTask({ ...emptyTask(), title: text.trim() });
  }

  /** 切换完成状态；转 done 时由后端补记完成时间 */
  async function toggleDone(task: Task): Promise<Task> {
    if (task.status === 'done') {
      return saveTask({ ...task, status: 'todo', completed_at: null });
    }
    return saveTask({ ...task, status: 'done', completed_at: null });
  }

  async function removeTask(taskId: string) {
    await service.deleteTask(taskId);
    tasks.value = tasks.value.filter(task => task.id !== taskId);
  }

  // ===== AI：一句话整理草稿 =====

  let draftSeq = 0;

  async function parseQuickAdd(input: string) {
    const text = input.trim();
    if (!text) {
      return;
    }
    // 允许新请求取代进行中的旧请求：过期结果由 taskId 检查丢弃
    draftSeq += 1;
    const taskId = `ai_${Date.now()}_${draftSeq}`;
    aiDraft.value = {
      taskId,
      input: text,
      status: 'generating',
      draft: emptyDraft(text),
      errorMessage: '',
    };

    try {
      const draft = await service.parseTaskText(text);
      if (aiDraft.value?.taskId !== taskId) {
        return; // 过期结果丢弃
      }
      aiDraft.value = { taskId, input: text, status: 'success', draft, errorMessage: '' };
    } catch (error) {
      if (aiDraft.value?.taskId !== taskId) {
        return;
      }
      aiDraft.value = {
        taskId,
        input: text,
        status: 'error',
        draft: emptyDraft(text),
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function discardDraft() {
    aiDraft.value = null;
  }

  /** 把（可能被用户改过的）草稿落库 */
  async function applyDraft(draft: TaskDraft): Promise<Task> {
    const created = await saveTask({ ...emptyTask(), ...draft, title: draft.title.trim() });
    aiDraft.value = null;
    return created;
  }

  // ===== AI：今日建议 =====

  async function loadPlan() {
    if (aiPlan.value?.status === 'loading') {
      return;
    }
    aiPlan.value = { status: 'loading', text: '', errorMessage: '' };
    try {
      const text = await service.suggestTaskPlan(tasks.value);
      aiPlan.value = { status: 'ready', text, errorMessage: '' };
    } catch (error) {
      aiPlan.value = {
        status: 'error',
        text: '',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function dismissPlan() {
    aiPlan.value = null;
  }

  return {
    // 状态
    tasks,
    tasksLoading,
    tasksLoaded,
    tasksError,
    aiDraft,
    aiPlan,
    // 派生
    openTasks,
    // 动作
    loadTasks,
    saveTask,
    createFromText,
    toggleDone,
    removeTask,
    parseQuickAdd,
    discardDraft,
    applyDraft,
    loadPlan,
    dismissPlan,
  };
}

export type TasksStore = ReturnType<typeof createTasksStore>;

export const useTasksStore = defineStore('tasks', () => createTasksStore());

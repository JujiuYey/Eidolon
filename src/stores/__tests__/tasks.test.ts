import { createPinia, defineStore, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createTasksStore } from '@/stores/tasks';
import type { TasksService } from '@/stores/tasks';
import type { Task, TaskDraft, TaskPlan } from '@/types/tasks';

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task_1',
    title: '写周报',
    notes: '',
    status: 'todo',
    priority: 'none',
    due_date: '',
    start_at: '',
    end_at: '',
    estimate_minutes: 0,
    source: 'manual',
    sort: 0,
    completed_at: null,
    created_at: 100,
    updated_at: 100,
    ...overrides,
  };
}

function makeDraft(overrides: Partial<TaskDraft> = {}): TaskDraft {
  return { title: '整理周报数据', priority: 'high', due_date: '2026-10-16', ...overrides };
}

function makePlan(overrides: Partial<TaskPlan> = {}): TaskPlan {
  return {
    blocks: [
      {
        task_id: 'task_1',
        title: '写周报',
        priority: 'high',
        start_at: '2026-10-14T09:00',
        end_at: '2026-10-14T10:30',
      },
    ],
    note: '先清周报',
    ...overrides,
  };
}

function bindStore(service: TasksService) {
  const useBound = defineStore('tasks-test', () => createTasksStore({ service }));
  return useBound();
}

function createService(overrides: Partial<TasksService> = {}): TasksService {
  return {
    listTasks: vi.fn().mockResolvedValue([]),
    upsertTask: vi.fn().mockImplementation(async (task: Task) => ({ ...task, id: task.id || 'task_new' })),
    deleteTask: vi.fn().mockResolvedValue('task_1'),
    parseTaskText: vi.fn().mockResolvedValue(makeDraft()),
    suggestTaskPlan: vi.fn().mockResolvedValue(makePlan()),
    ...overrides,
  };
}

describe('tasks store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loadTasks fills the list and clears stale errors', async () => {
    const service = createService({ listTasks: vi.fn().mockResolvedValue([makeTask()]) });
    const store = bindStore(service);

    store.tasksError = '旧错误';
    await store.loadTasks();

    expect(store.tasks).toHaveLength(1);
    expect(store.tasksLoaded).toBe(true);
    expect(store.tasksError).toBe('');
    expect(store.tasksLoading).toBe(false);
  });

  it('loadTasks records errors without throwing', async () => {
    const service = createService({ listTasks: vi.fn().mockRejectedValue('数据库打不开') });
    const store = bindStore(service);

    await store.loadTasks();

    expect(store.tasksError).toBe('数据库打不开');
    expect(store.tasksLoaded).toBe(false);
  });

  it('createFromText stores the raw text as title', async () => {
    const service = createService();
    const store = bindStore(service);

    await store.createFromText('  记得回评审意见  ');

    expect(service.upsertTask).toHaveBeenCalledWith(
      expect.objectContaining({ title: '记得回评审意见', status: 'todo' }),
    );
    expect(store.tasks).toHaveLength(1);
  });

  it('toggleDone flips status and reverts completed_at to null for the backend to manage', async () => {
    const service = createService({ listTasks: vi.fn().mockResolvedValue([makeTask()]) });
    const store = bindStore(service);
    await store.loadTasks();

    const done = await store.toggleDone(store.tasks[0]!);
    expect(done.status).toBe('done');
    expect(done.completed_at).toBeNull();

    const reopened = await store.toggleDone(done);
    expect(reopened.status).toBe('todo');
    expect(reopened.completed_at).toBeNull();
  });

  it('removeTask calls the service and drops the task locally', async () => {
    const service = createService({ listTasks: vi.fn().mockResolvedValue([makeTask()]) });
    const store = bindStore(service);
    await store.loadTasks();

    await store.removeTask('task_1');

    expect(service.deleteTask).toHaveBeenCalledWith('task_1');
    expect(store.tasks).toHaveLength(0);
  });

  it('parseQuickAdd builds a generating candidate then fills the parsed draft', async () => {
    const service = createService({ parseTaskText: vi.fn().mockResolvedValue(makeDraft()) });
    const store = bindStore(service);

    const pending = store.parseQuickAdd('周五前整理周报，优先级高');
    expect(store.aiDraft?.status).toBe('generating');
    await pending;

    expect(store.aiDraft?.status).toBe('success');
    expect(store.aiDraft?.draft.title).toBe('整理周报数据');
    expect(store.aiDraft?.draft.priority).toBe('high');
  });

  it('parseQuickAdd keeps the error on the candidate instead of throwing', async () => {
    const service = createService({ parseTaskText: vi.fn().mockRejectedValue('请先配置任务助手模型') });
    const store = bindStore(service);

    await store.parseQuickAdd('做点事');

    expect(store.aiDraft?.status).toBe('error');
    expect(store.aiDraft?.errorMessage).toBe('请先配置任务助手模型');
  });

  it('parseQuickAdd ignores stale results after a newer request', async () => {
    let resolveFirst: ((draft: TaskDraft) => void) | undefined;
    const service = createService({
      parseTaskText: vi
        .fn()
        .mockImplementationOnce(
          () =>
            new Promise<TaskDraft>(resolve => {
              resolveFirst = resolve;
            }),
        )
        .mockResolvedValue(makeDraft({ title: '第二个' })),
    });
    const store = bindStore(service);

    const first = store.parseQuickAdd('第一件事');
    await store.parseQuickAdd('第二件事');
    resolveFirst?.(makeDraft({ title: '迟到的第一个' }));
    await first;

    expect(store.aiDraft?.draft.title).toBe('第二个');
  });

  it('applyDraft persists the (possibly edited) draft and clears the candidate', async () => {
    const service = createService({ parseTaskText: vi.fn().mockResolvedValue(makeDraft()) });
    const store = bindStore(service);
    await store.parseQuickAdd('一句话');

    const created = await store.applyDraft({ ...makeDraft(), title: '改过的标题', priority: 'low' });

    expect(service.upsertTask).toHaveBeenCalledWith(expect.objectContaining({ title: '改过的标题', priority: 'low' }));
    expect(created.title).toBe('改过的标题');
    expect(store.aiDraft).toBeNull();
    expect(store.tasks).toHaveLength(1);
  });

  it('loadPlan stores the structured plan and dismissPlan clears it', async () => {
    const service = createService({
      listTasks: vi.fn().mockResolvedValue([makeTask()]),
      suggestTaskPlan: vi.fn().mockResolvedValue(makePlan({ note: '先清逾期' })),
    });
    const store = bindStore(service);
    await store.loadTasks();

    await store.loadPlan();
    expect(store.aiPlan?.status).toBe('ready');
    expect(store.aiPlan?.note).toBe('先清逾期');
    expect(store.aiPlan?.blocks).toHaveLength(1);
    expect(store.aiPlan?.blocks[0]).toMatchObject({ task_id: 'task_1', start_at: '2026-10-14T09:00' });

    store.dismissPlan();
    expect(store.aiPlan).toBeNull();
  });

  it('loadPlan records errors without throwing', async () => {
    const service = createService({
      listTasks: vi.fn().mockResolvedValue([makeTask()]),
      suggestTaskPlan: vi.fn().mockRejectedValue('当前没有需要排期的任务'),
    });
    const store = bindStore(service);
    await store.loadTasks();

    await store.loadPlan();

    expect(store.aiPlan?.status).toBe('error');
    expect(store.aiPlan?.errorMessage).toBe('当前没有需要排期的任务');
  });

  it('applyPlan writes blocks back to tasks and clears the candidate', async () => {
    const service = createService({
      listTasks: vi.fn().mockResolvedValue([makeTask(), makeTask({ id: 'task_2', title: '回复评审' })]),
      suggestTaskPlan: vi.fn().mockResolvedValue(
        makePlan({
          blocks: [
            {
              task_id: 'task_1',
              title: '写周报',
              priority: 'high',
              start_at: '2026-10-14T09:00',
              end_at: '2026-10-14T10:30',
            },
            {
              task_id: 'task_missing',
              title: '幽灵任务',
              priority: 'none',
              start_at: '2026-10-15T09:00',
              end_at: '2026-10-15T10:00',
            },
          ],
        }),
      ),
    });
    const store = bindStore(service);
    await store.loadTasks();
    await store.loadPlan();

    const applied = await store.applyPlan();

    expect(applied).toBe(1); // 只有存在的任务被应用
    expect(service.upsertTask).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'task_1', start_at: '2026-10-14T09:00', end_at: '2026-10-14T10:30' }),
    );
    expect(store.tasks.find(task => task.id === 'task_1')?.start_at).toBe('2026-10-14T09:00');
    expect(store.aiPlan).toBeNull();
  });

  it('applyPlan keeps the candidate when every block fails', async () => {
    const service = createService({
      listTasks: vi.fn().mockResolvedValue([makeTask()]),
      upsertTask: vi.fn().mockRejectedValue('数据库不可写'),
    });
    const store = bindStore(service);
    await store.loadTasks();
    await store.loadPlan();

    const applied = await store.applyPlan();

    expect(applied).toBe(0);
    expect(store.aiPlan?.status).toBe('ready'); // 失败时建议保留，用户可重试
  });
});

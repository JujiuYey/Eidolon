-- 个人任务管理器
-- 轻量任务清单：标题/备注/状态/优先级/截止日期
-- 时间戳字段统一使用 Unix 毫秒（UTC）；
-- due_date 是日历日期而非时间点，用 'YYYY-MM-DD' 文本存储，避免时区换算

-- ============================================================
-- 任务
-- ============================================================
CREATE TABLE tasks (
    -- 任务主键，格式 task_<nanoid>
    id TEXT PRIMARY KEY,
    -- 任务标题
    title TEXT NOT NULL,
    -- 补充备注（纯文本，可多行）
    notes TEXT NOT NULL DEFAULT '',
    -- 状态：todo 待办 | doing 进行中 | done 已完成
    status TEXT NOT NULL DEFAULT 'todo',
    -- 优先级：high | medium | low | none
    priority TEXT NOT NULL DEFAULT 'none',
    -- 截止日期（YYYY-MM-DD），NULL 表示无截止
    due_date TEXT,
    -- 来源：manual 手工创建（预留禅道等外部来源）
    source TEXT NOT NULL DEFAULT 'manual',
    -- 展示顺序，越小越靠前（预留手动排序）
    sort INTEGER NOT NULL DEFAULT 0,
    -- 完成时间，Unix 毫秒；未完成为 NULL
    completed_at INTEGER,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_tasks_status ON tasks (status);
CREATE INDEX idx_tasks_sort ON tasks (sort);

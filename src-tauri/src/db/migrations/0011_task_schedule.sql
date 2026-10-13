-- 任务排期（时间块）
-- 一个任务最多一个时间块：start_at / end_at 成对出现，空串 = 未排期（拖离日程即清空）。
-- 时间为本地无时区 ISO 文本（YYYY-MM-DDTHH:MM），与 due_date 同思路，避免时区换算；
-- estimate_minutes 是预估时长（分钟），拖入日历时决定默认块大小，AI 排期也参考它，0 = 未设置。

ALTER TABLE tasks ADD COLUMN start_at TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN end_at TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN estimate_minutes INTEGER NOT NULL DEFAULT 0;

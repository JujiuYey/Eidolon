-- 周报功能数据结构
-- 本地 git 仓库列表与生成的周报历史两张表；提交聚合在前端完成
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- 周报仓库：参与周报统计的本地 git 仓库
-- ============================================================
CREATE TABLE wr_repo (
    -- 仓库主键，格式 wrre_<nanoid>
    id TEXT PRIMARY KEY,
    -- 仓库绝对路径，唯一（同一仓库只添加一次）
    path TEXT NOT NULL UNIQUE,
    -- 展示名，默认取目录名
    name TEXT NOT NULL,
    -- 展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 仓库列表按 sort 展示
CREATE INDEX idx_wr_repo_sort ON wr_repo (sort);

-- ============================================================
-- 周报历史：每次保存的周报正文与统计区间
-- ============================================================
CREATE TABLE wr_report (
    -- 周报主键，格式 wrpt_<nanoid>
    id TEXT PRIMARY KEY,
    -- 周报标题（如 "周报 2026.09.28 - 2026.10.04"）
    title TEXT NOT NULL,
    -- 统计区间起点，Unix 毫秒
    range_start INTEGER NOT NULL,
    -- 统计区间终点，Unix 毫秒
    range_end INTEGER NOT NULL,
    -- Markdown 正文
    content TEXT NOT NULL DEFAULT '',
    -- 生成时参与的仓库数
    repo_count INTEGER NOT NULL DEFAULT 0,
    -- 生成时聚合的提交数
    commit_count INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 历史列表按创建时间倒序
CREATE INDEX idx_wr_report_created_at ON wr_report (created_at DESC);

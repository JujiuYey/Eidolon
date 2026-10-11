-- 数据字典：表结构快照管理
-- 不连数据库；用户粘贴 DDL 或"智能查询"结果，解析成结构化快照落库，
-- 相邻快照间计算变更时间线。
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- 库条目：数据字典的顶层容器（如 "病案质控库"）
-- ============================================================
CREATE TABLE schema_databases (
    -- 主键，格式 sdb_<nanoid>
    id TEXT PRIMARY KEY,
    -- 显示名（如 "病案质控库"）
    name TEXT NOT NULL,
    -- 方言：mysql | pg | oracle（决定 DDL 解析方言与查询模板）
    db_type TEXT NOT NULL DEFAULT 'mysql',
    -- 备注说明
    note TEXT NOT NULL DEFAULT '',
    -- 展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- ============================================================
-- 快照：某库在某时点的完整表结构
-- ============================================================
CREATE TABLE schema_snapshots (
    -- 主键，格式 ssn_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所属库，随库级联删除
    database_id TEXT NOT NULL REFERENCES schema_databases (id) ON DELETE CASCADE,
    -- 版本名（如 "2026-10 升级前"）
    label TEXT NOT NULL,
    -- 导入来源：ddl 粘贴 DDL | query 粘贴查询结果
    source_kind TEXT NOT NULL DEFAULT 'ddl',
    -- 原始粘贴文本（审计用）
    source_text TEXT NOT NULL DEFAULT '',
    -- 解析后的结构化 JSON（ParsedSchema）
    tables_json TEXT NOT NULL,
    -- 快照备注
    note TEXT NOT NULL DEFAULT '',
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_schema_snapshots_db ON schema_snapshots (database_id, created_at);

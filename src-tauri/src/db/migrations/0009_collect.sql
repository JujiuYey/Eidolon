-- 收藏夹（收集盒）
-- 链接与随手记事统一存 collect_items，kind 区分类型；
-- 链接的标题/摘要/favicon 由后端抓取回填，content 兼作用户备注（链接）与正文（记事）
-- 时间戳字段统一使用 Unix 毫秒（UTC）

CREATE TABLE collect_items (
    -- 主键，格式 collect_<nanoid>
    id TEXT PRIMARY KEY,
    -- 类型：link 链接 | note 记事
    kind TEXT NOT NULL CHECK (kind IN ('link', 'note')),
    -- 链接地址；记事为 NULL
    url TEXT,
    -- 标题：链接为抓取/手填标题；记事为 NULL
    title TEXT,
    -- 链接摘要（抓取的 description）；记事为 NULL
    description TEXT,
    -- 用户备注（链接）/ 正文（记事）
    content TEXT NOT NULL DEFAULT '',
    -- 链接图标地址；记事为 NULL
    favicon_url TEXT,
    -- 是否已归档：0 否 | 1 是
    archived INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_collect_items_archived ON collect_items (archived, created_at DESC);

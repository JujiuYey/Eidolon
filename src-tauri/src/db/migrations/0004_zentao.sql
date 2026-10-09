-- 禅道集成数据结构
-- 仅落地账户配置；任务/Bug 数据每次实时拉取，不落库
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- 禅道账户：站点地址 + 登录凭据
-- ============================================================
CREATE TABLE zentao_accounts (
    -- 账户主键，格式 zta_<nanoid>
    id TEXT PRIMARY KEY,
    -- 账户显示名（如 "公司禅道"）
    name TEXT NOT NULL,
    -- 站点根地址（如 http://192.168.10.209，可带子路径）
    base_url TEXT NOT NULL,
    -- 登录账号
    account TEXT NOT NULL,
    -- 登录密码；遵循项目"掩码而非加密"约定，明文存储
    password TEXT NOT NULL DEFAULT '',
    -- 是否启用（0/1）
    enabled INTEGER NOT NULL DEFAULT 1,
    -- 展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 账户列表按 sort 展示
CREATE INDEX idx_zentao_accounts_sort ON zentao_accounts (sort);

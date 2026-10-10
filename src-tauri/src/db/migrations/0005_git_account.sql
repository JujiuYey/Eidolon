-- Git 账号：用户在本机的各提交身份（多台电脑可能使用不同邮箱）
-- 周报「只看我的提交」按启用账号的邮箱匹配提交作者
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- Git 账号：显示名 + Git 邮箱
-- ============================================================
CREATE TABLE git_accounts (
    -- 账户主键，格式 gacc_<nanoid>
    id TEXT PRIMARY KEY,
    -- 账户显示名（如 "公司电脑"）
    name TEXT NOT NULL,
    -- Git 提交邮箱，周报按此匹配作者
    email TEXT NOT NULL,
    -- 是否启用（0/1）；停用的账号不参与周报匹配
    enabled INTEGER NOT NULL DEFAULT 1,
    -- 展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 账户列表按 sort 展示
CREATE INDEX idx_git_accounts_sort ON git_accounts (sort);

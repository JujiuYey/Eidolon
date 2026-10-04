-- 邮件发送功能数据结构
-- SMTP 账户、邮件模板、发送历史三张表；模板渲染在前端完成，后端只存原文
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- SMTP 账户：发信所需的 SMTP 连接与凭据
-- ============================================================
CREATE TABLE smtp_accounts (
    -- 账户主键，格式 sacc_<nanoid>
    id TEXT PRIMARY KEY,
    -- 账户显示名（如 "公司邮箱"）
    name TEXT NOT NULL,
    -- 发件邮箱地址，同时作为 SMTP 登录用户名
    email TEXT NOT NULL,
    -- SMTP 密码/授权码；遵循项目“掩码而非加密”约定，明文存储
    password TEXT NOT NULL DEFAULT '',
    -- SMTP 服务器地址（如 smtp.qiye.aliyun.com）
    host TEXT NOT NULL,
    -- SMTP 端口（1-65535）
    port INTEGER NOT NULL,
    -- 加密方式：none/starttls/tls（tls 为 465 直连 TLS）
    encryption TEXT NOT NULL DEFAULT 'tls',
    -- 发件人显示名，可空；为空时发件头只有邮箱地址
    from_name TEXT NOT NULL DEFAULT '',
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
CREATE INDEX idx_smtp_accounts_sort ON smtp_accounts (sort);

-- ============================================================
-- 邮件模板：主题与正文含 {{变量}} 占位符（支持中文变量名）
-- ============================================================
CREATE TABLE mail_templates (
    -- 模板主键，格式 mtpl_<nanoid>；内置模板使用 mtpl_builtin_ 前缀固定 ID
    id TEXT PRIMARY KEY,
    -- 模板名称（如 "请假申请"）
    name TEXT NOT NULL,
    -- 主题模板，含 {{变量}}
    subject TEXT NOT NULL DEFAULT '',
    -- 正文模板，含 {{变量}}
    body TEXT NOT NULL DEFAULT '',
    -- 展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 模板列表按 sort 展示
CREATE INDEX idx_mail_templates_sort ON mail_templates (sort);

-- 内置模板：固定 ID + INSERT OR IGNORE，仅在首次建库时写入；
-- 用户删除后不会随重启恢复（迁移只执行一次）
INSERT OR IGNORE INTO mail_templates (id, name, subject, body, sort, created_at, updated_at) VALUES
(
    'mtpl_builtin_leave',
    '请假申请',
    '{{姓名}}的{{请假类型}}申请',
    '尊敬的{{审批人}}：

您好！

我因{{请假事由}}，需请假{{请假天数}}天，时间为{{开始日期}}至{{结束日期}}。请假期间工作已安排交接，如有紧急事项请随时与我联系。

恳请批准，谢谢！

此致
敬礼

{{姓名}}
{{申请日期}}',
    0,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000
),
(
    'mtpl_builtin_comp_time',
    '调休申请',
    '{{姓名}}的调休申请',
    '尊敬的{{审批人}}：

您好！

我于{{加班日期}}加班{{加班时长}}，现申请于{{调休日期}}调休{{调休天数}}天，调休期间工作已安排交接。

恳请批准，谢谢！

此致
敬礼

{{姓名}}
{{申请日期}}',
    1,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000
),
(
    'mtpl_builtin_notice',
    '通用通知',
    '【通知】{{主题}}',
    '各位同事：

好！

{{通知内容}}

如有疑问，请随时与我联系。

{{姓名}}
{{发送日期}}',
    2,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000
);

-- ============================================================
-- 发送历史：每次发送（含失败）落一条，仓库层只保留最近 500 条
-- ============================================================
CREATE TABLE sent_emails (
    -- 历史主键，格式 sent_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所用账户；删除账户时置空，历史本身保留
    account_id TEXT REFERENCES smtp_accounts (id) ON DELETE SET NULL,
    -- 发件邮箱快照，账户删除后仍可展示
    account_email TEXT NOT NULL DEFAULT '',
    -- 收件人列表（逗号分隔）
    to_addresses TEXT NOT NULL DEFAULT '',
    -- 抄送列表（逗号分隔），可为空
    cc_addresses TEXT NOT NULL DEFAULT '',
    -- 发送的主题（渲染后的最终文本）
    subject TEXT NOT NULL DEFAULT '',
    -- 发送的正文（渲染后的最终文本）
    body TEXT NOT NULL DEFAULT '',
    -- 使用的模板；删除模板时置空
    template_id TEXT REFERENCES mail_templates (id) ON DELETE SET NULL,
    -- 发送状态：sent/failed
    status TEXT NOT NULL,
    -- 失败原因，成功时为 NULL
    error_message TEXT,
    -- 发送时间，Unix 毫秒
    sent_at INTEGER NOT NULL,
    -- 记录创建时间，Unix 毫秒
    created_at INTEGER NOT NULL
);

-- 历史列表按发送时间倒序
CREATE INDEX idx_sent_emails_sent_at ON sent_emails (sent_at DESC);

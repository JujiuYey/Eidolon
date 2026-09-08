-- API Client 首版数据结构
-- 关联、排序、时间字段独立成列；灵活结构（Query/Headers/Body/Variables/Snapshot）以 JSON 文本保存
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- 项目：接口请求工具的顶层容器
-- ============================================================
CREATE TABLE api_projects (
    -- 项目主键，格式 apj_<nanoid>
    id TEXT PRIMARY KEY,
    -- 项目名称，用户可见
    name TEXT NOT NULL,
    -- 项目描述，可选
    description TEXT NOT NULL DEFAULT '',
    -- 同级项目中的展示顺序，越小越靠前
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- ============================================================
-- 分组：项目内的二级目录，首版只允许一层（parent_group_id 恒为 NULL）
-- ============================================================
CREATE TABLE api_groups (
    -- 分组主键，格式 agr_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所属项目；删除项目时级联清除
    project_id TEXT NOT NULL REFERENCES api_projects (id) ON DELETE CASCADE,
    -- 预留多层分组（首版始终为 NULL，UI 不开放）；自引用并级联删除
    parent_group_id TEXT REFERENCES api_groups (id) ON DELETE CASCADE,
    -- 分组名称
    name TEXT NOT NULL,
    -- 同项目内的展示顺序
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 按项目过滤 + 排序的常用查询
CREATE INDEX idx_api_groups_project ON api_groups (project_id, sort);

-- ============================================================
-- 请求：归属分组（再经分组 → 项目）；保存草稿与执行定义
-- ============================================================
CREATE TABLE api_requests (
    -- 请求主键，格式 areq_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所属分组；删除分组时级联清除
    group_id TEXT NOT NULL REFERENCES api_groups (id) ON DELETE CASCADE,
    -- 请求名称
    name TEXT NOT NULL,
    -- HTTP 方法（GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS），由 normalize_method 校验
    method TEXT NOT NULL DEFAULT 'GET',
    -- 请求地址模板，含 {{变量}}；发送时由 prepare_request 解析
    url TEXT NOT NULL DEFAULT '',
    -- Query 字符串键值行数组 JSON；保留顺序、允许重复键和 disabled 标志
    query_json TEXT NOT NULL DEFAULT '[]',
    -- 请求头键值行数组 JSON；同上
    headers_json TEXT NOT NULL DEFAULT '[]',
    -- 请求体类型：none/json/text/form
    body_kind TEXT NOT NULL DEFAULT 'none',
    -- JSON/Text 类型时的请求体原文（含变量替换前的模板）
    body_text TEXT NOT NULL DEFAULT '',
    -- form 类型时的 URL 编码键值行数组 JSON
    body_form_json TEXT NOT NULL DEFAULT '[]',
    -- 单次执行超时（毫秒），默认 30000
    timeout_ms INTEGER NOT NULL DEFAULT 30000,
    -- AI 生成要求（提示词），随请求定义保存以便复用
    ai_prompt TEXT NOT NULL DEFAULT '',
    -- AI 参考内容（字段说明/文档片段），随请求定义保存
    ai_reference TEXT NOT NULL DEFAULT '',
    -- 同分组内的展示顺序
    sort INTEGER NOT NULL DEFAULT 0,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 按分组过滤 + 排序的常用查询
CREATE INDEX idx_api_requests_group ON api_requests (group_id, sort);

-- ============================================================
-- 环境：项目级的 base_url 与变量键值，供发送时变量解析
-- ============================================================
CREATE TABLE api_environments (
    -- 环境主键，格式 aenv_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所属项目；删除项目时级联清除
    project_id TEXT NOT NULL REFERENCES api_projects (id) ON DELETE CASCADE,
    -- 环境名称（如 "测试环境"、"预发环境"）
    name TEXT NOT NULL,
    -- 环境基础地址；相对 URL 在此基础上拼接
    base_url TEXT NOT NULL DEFAULT '',
    -- 变量键值行数组 JSON；含 enabled 标志和 {{变量}} 引用
    variables_json TEXT NOT NULL DEFAULT '[]',
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 按项目 + 名称查找
CREATE INDEX idx_api_environments_project ON api_environments (project_id, name);

-- ============================================================
-- 执行历史：每次发送的快照；敏感字段已脱敏、响应体受 1 MiB 上限
-- ============================================================
CREATE TABLE api_request_histories (
    -- 历史主键，格式 ahis_<nanoid>
    id TEXT PRIMARY KEY,
    -- 关联的请求；删除请求时级联清除
    request_id TEXT NOT NULL REFERENCES api_requests (id) ON DELETE CASCADE,
    -- 执行时所选环境名快照（仅名称，不含凭据）
    environment_name TEXT,
    -- 发送时冻结的请求快照 JSON（method/url/query/headers/body/timeout/env_name）
    request_snapshot_json TEXT NOT NULL DEFAULT '{}',
    -- 执行状态：success/http_error/network_error/timeout/cancelled/oversize
    status TEXT NOT NULL,
    -- HTTP 状态码；连接失败/超时/取消时为 NULL
    status_code INTEGER,
    -- 响应头键值行数组 JSON（已脱敏）
    response_headers_json TEXT NOT NULL DEFAULT '[]',
    -- 响应体预览（受 MAX_HISTORY_BODY_BYTES 上限截断；JSON 被截断后按文本展示）
    response_body_preview TEXT NOT NULL DEFAULT '',
    -- 响应体是否被截断的布尔标志（0/1）
    response_body_truncated INTEGER NOT NULL DEFAULT 0,
    -- 客户端总耗时，毫秒
    duration_ms INTEGER NOT NULL DEFAULT 0,
    -- 失败原因（超时/连接错误/取消等）
    error_message TEXT,
    -- 执行时间，Unix 毫秒
    executed_at INTEGER NOT NULL
);

-- 按请求查询最新历史的常用索引
CREATE INDEX idx_api_histories_request ON api_request_histories (request_id, executed_at DESC);
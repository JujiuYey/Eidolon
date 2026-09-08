-- API Client 首版数据结构
-- 关联、排序、时间字段独立成列；灵活结构以 JSON 文本保存

CREATE TABLE api_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    sort INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE api_groups (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES api_projects (id) ON DELETE CASCADE,
    -- 预留多层分组，首版始终为 NULL
    parent_group_id TEXT REFERENCES api_groups (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    sort INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_api_groups_project ON api_groups (project_id, sort);

-- 请求通过分组归属项目，不重复维护项目 ID
CREATE TABLE api_requests (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES api_groups (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    method TEXT NOT NULL DEFAULT 'GET',
    url TEXT NOT NULL DEFAULT '',
    query_json TEXT NOT NULL DEFAULT '[]',
    headers_json TEXT NOT NULL DEFAULT '[]',
    body_kind TEXT NOT NULL DEFAULT 'none',
    body_text TEXT NOT NULL DEFAULT '',
    body_form_json TEXT NOT NULL DEFAULT '[]',
    timeout_ms INTEGER NOT NULL DEFAULT 30000,
    ai_prompt TEXT NOT NULL DEFAULT '',
    ai_reference TEXT NOT NULL DEFAULT '',
    sort INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_api_requests_group ON api_requests (group_id, sort);

CREATE TABLE api_environments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES api_projects (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL DEFAULT '',
    variables_json TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_api_environments_project ON api_environments (project_id, name);

-- 执行历史：请求快照与响应体均已脱敏、截断
CREATE TABLE api_request_histories (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES api_requests (id) ON DELETE CASCADE,
    environment_name TEXT,
    request_snapshot_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL,
    status_code INTEGER,
    response_headers_json TEXT NOT NULL DEFAULT '[]',
    response_body_preview TEXT NOT NULL DEFAULT '',
    response_body_truncated INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    executed_at INTEGER NOT NULL
);

CREATE INDEX idx_api_histories_request ON api_request_histories (request_id, executed_at DESC);

-- 删除项目时的关联计数需要在事务内读取，这里保存最近一次删除的统计供 UI 展示
CREATE TABLE api_project_deletions (
    project_id TEXT PRIMARY KEY,
    group_count INTEGER NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    history_count INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER NOT NULL
);

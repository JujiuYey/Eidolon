-- 代码导入：把后端代码仓库的接口清单导入为 API 请求
-- 一个项目绑定一个导入源（框架类型 + 仓库路径），重复导入按 replace 语义重建分组
-- 时间字段统一使用 Unix 毫秒（UTC）

-- ============================================================
-- 导入源：项目与后端仓库的绑定关系
-- ============================================================
CREATE TABLE api_import_sources (
    -- 导入源主键，格式 aimp_<nanoid>
    id TEXT PRIMARY KEY,
    -- 所属 API 项目，删除项目时一并删除
    project_id TEXT NOT NULL REFERENCES api_projects (id) ON DELETE CASCADE,
    -- 后端框架类型，决定导出命令模板（当前支持 vef-go）
    framework TEXT NOT NULL,
    -- 后端代码仓库的本地绝对路径
    repo_path TEXT NOT NULL,
    -- 创建时间，Unix 毫秒
    created_at INTEGER NOT NULL,
    -- 最近一次更新时间，Unix 毫秒
    updated_at INTEGER NOT NULL
);

-- 每个项目最多一个导入源；按项目查询
CREATE UNIQUE INDEX idx_api_import_sources_project ON api_import_sources (project_id);

-- ============================================================
-- 请求表补充导入身份：手工创建的请求为 NULL，
-- 导入生成的请求记录来源接口（如 smp/sys/user#find_page），
-- 用于识别「这条请求来自代码导入」以及将来的增量同步
-- ============================================================
ALTER TABLE api_requests ADD COLUMN external_key TEXT;

CREATE INDEX idx_api_requests_external_key ON api_requests (external_key);

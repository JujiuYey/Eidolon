-- 数据字典功能已移除：快照/库表不再使用
DROP INDEX IF EXISTS idx_schema_snapshots_db;
DROP TABLE IF EXISTS schema_snapshots;
DROP TABLE IF EXISTS schema_databases;

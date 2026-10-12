-- 收藏夹软删除：删除先进回收站，保留 30 天后由应用在拉取列表时物理清除；
-- deleted_at 为 NULL 表示正常条目，非 NULL 为删除时间（Unix 毫秒）

ALTER TABLE collect_items ADD COLUMN deleted_at INTEGER;

CREATE INDEX idx_collect_items_deleted ON collect_items (deleted_at);

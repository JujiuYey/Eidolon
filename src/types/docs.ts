/** 文档管理设置（镜像后端 models/docs.rs，snake_case） */
export interface DocsSettings {
  root_dir: string | null;
}

/** 文档库条目，path 为相对根目录的 `/` 分隔路径 */
export interface DocsEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  updated_at: number;
}

/** 全文搜索命中行 */
export interface DocsSearchResult {
  path: string;
  name: string;
  line_number: number;
  line_text: string;
}

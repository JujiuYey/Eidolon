/** 一个文档库，id 由后端按路径确定性哈希生成，可作路由参数 */
export interface DocsRoot {
  id: string;
  path: string;
}

/** 文档管理设置（镜像后端 models/docs.rs，snake_case） */
export interface DocsSettings {
  /** 旧版单根字段，仅用于兼容旧配置文件，新写入恒为 null */
  root_dir: string | null;
  roots: DocsRoot[];
}

/** 文档库条目，path 为相对所在文档库根目录的 `/` 分隔路径 */
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

export type CollectKind = 'link' | 'note';

export interface CollectItem {
  id: string;
  kind: CollectKind;
  /** 链接地址；记事为 null */
  url: string | null;
  /** 标题：链接为抓取/手填标题；记事为 null */
  title: string | null;
  /** 链接摘要（抓取的 description）；记事为 null */
  description: string | null;
  /** 用户备注（链接）/ 正文（记事） */
  content: string;
  /** 链接图标地址；记事为 null */
  favicon_url: string | null;
  archived: boolean;
  created_at: number;
  updated_at: number;
  /** 软删除时间（Unix 毫秒）；null 表示未删除，保留 30 天后物理清除 */
  deleted_at: number | null;
}

/** 链接元数据抓取结果 */
export interface LinkMetadata {
  title: string | null;
  description: string | null;
  favicon_url: string | null;
}

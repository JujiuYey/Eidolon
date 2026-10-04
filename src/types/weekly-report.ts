/**
 * 周报功能的 Tauri DTO 类型。
 *
 * 与后端 `src-tauri/src/models/weekly_report.rs` 的 serde 结构一一对应，
 * 字段保持 snake_case 直映（同 `types/mail.ts` 约定）。
 */

/** 作者过滤方式 */
export type WeeklyReportAuthorMode = 'auto' | 'custom' | 'all';

/** 参与周报统计的本地 git 仓库 */
export interface WeeklyReportRepo {
  id: string;
  /** 仓库绝对路径 */
  path: string;
  /** 展示名，默认取目录名 */
  name: string;
  sort: number;
  created_at: number;
  updated_at: number;
}

/** 单条提交记录（周报聚合的原始素材） */
export interface WeeklyReportCommit {
  /** 完整哈希，跨引用去重的键 */
  hash: string;
  short_hash: string;
  /** 提交主题（首行） */
  subject: string;
  author_name: string;
  /** 作者时间，Unix 毫秒 */
  timestamp: number;
}

/** 单个仓库的提交拉取结果。拉取失败不中断整体，错误放在 error 里 */
export interface RepoCommitLog {
  repo_id: string;
  repo_name: string;
  commits: WeeklyReportCommit[];
  error: string | null;
}

/** 提交拉取请求。区间为 Unix 毫秒（含两端） */
export interface FetchCommitsRequest {
  /** 要拉取的仓库 ID；为空表示全部仓库 */
  repo_ids: string[];
  since_ms: number;
  until_ms: number;
  author_mode: WeeklyReportAuthorMode;
  /** author_mode = 'custom' 时生效，匹配作者姓名或邮箱 */
  author: string;
}

/** 保存的周报历史 */
export interface WeeklyReportEntry {
  id: string;
  title: string;
  /** 统计区间起点，Unix 毫秒 */
  range_start: number;
  /** 统计区间终点，Unix 毫秒 */
  range_end: number;
  /** Markdown 正文 */
  content: string;
  repo_count: number;
  commit_count: number;
  created_at: number;
  updated_at: number;
}

/** AI 润色请求。草稿是基于提交记录聚合好的 Markdown */
export interface PolishWeeklyReportInput {
  title: string;
  draft: string;
}

/** AI 润色结果 */
export interface PolishWeeklyReportResult {
  content: string;
  model_label: string;
}

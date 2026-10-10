/**
 * 禅道功能的 Tauri DTO 类型。
 *
 * 与后端 `src-tauri/src/models/zentao.rs` 的 serde 结构一一对应，
 * 字段保持 snake_case 直映（同 `types/mail.ts` 约定）。
 */

export interface ZentaoAccount {
  id: string;
  /** 账户显示名（如 "公司禅道"） */
  name: string;
  /** 站点根地址，如 http://192.168.10.209，可带子路径 */
  base_url: string;
  /** 登录账号 */
  account: string;
  /** 登录密码 */
  password: string;
  enabled: boolean;
  sort: number;
  created_at: number;
  updated_at: number;
}

/** 禅道用户（后端已把字符串/对象多形态统一为对象） */
export interface ZentaoUser {
  account: string;
  realname: string;
}

/** 归属当前用户的进行中任务 */
export interface ZentaoTask {
  id: number;
  name: string;
  pri: number;
  /** wait / doing / pause */
  status: string;
  assigned_to: ZentaoUser;
  execution: number;
  execution_name: string;
  /** 截止日期，YYYY-MM-DD，可为空串 */
  deadline: string;
  /** 预计工时 */
  estimate: number;
  /** 剩余工时 */
  left: number;
  /** 已消耗工时 */
  consumed: number;
  /** 关联需求标题，可为空串 */
  story_title: string;
  /** 任务类型（devel/test/design 等） */
  task_type: string;
}

/** 归属当前用户的未关闭 Bug */
export interface ZentaoBug {
  id: number;
  title: string;
  severity: number;
  pri: number;
  /** active / resolved / delay / closed */
  status: string;
  assigned_to: ZentaoUser;
  resolution: string;
  opened_by: ZentaoUser;
  /** ISO 时间串 */
  opened_date: string;
  deadline: string;
  execution: number;
  execution_name: string;
}

/** 指派给当前用户的需求（研发/用户需求同结构，`story_type` 区分） */
export interface ZentaoStory {
  id: number;
  title: string;
  pri: number;
  /** draft / reviewing / active / changing / closed */
  status: string;
  /** 研发阶段：wait / projected / planned / developing / developed… */
  stage: string;
  /** story = 研发需求，requirement = 用户需求 */
  story_type: string;
  assigned_to: ZentaoUser;
  product: number;
  /** 所属产品名称，后端已从产品列表回填 */
  product_name: string;
  /** 预计工时 */
  estimate: number;
  opened_by: ZentaoUser;
  /** ISO 时间串 */
  opened_date: string;
}

/** "我的工作台"聚合结果 */
export interface ZentaoMyWork {
  account_id: string;
  tasks: ZentaoTask[];
  bugs: ZentaoBug[];
  /** 指派给我的需求（研发 + 用户两类） */
  stories: ZentaoStory[];
  /** 非致命告警（如个别执行拉取失败） */
  warnings: string[];
  /** 拉取完成时间，Unix 毫秒 */
  fetched_at: number;
}

export interface TestZentaoConnectionRequest {
  base_url: string;
  account: string;
  password: string;
}

export interface TestZentaoConnectionResponse {
  success: boolean;
  message: string;
}

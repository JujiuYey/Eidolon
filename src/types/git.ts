/**
 * Git 账户 DTO。与 `src-tauri/src/models/git.rs` 的 `GitAccount` 字段一一对应（snake_case 直映）。
 *
 * 每个账户代表用户的一个 git 提交身份（多台电脑可能使用不同邮箱）；
 * 周报「只看我的提交」按启用账户的邮箱匹配提交作者。
 */
export interface GitAccount {
  id: string;
  /** 账户显示名（如 "公司电脑"） */
  name: string;
  /** Git 提交邮箱，周报按此匹配作者 */
  email: string;
  enabled: boolean;
  sort: number;
  created_at: number;
  updated_at: number;
}

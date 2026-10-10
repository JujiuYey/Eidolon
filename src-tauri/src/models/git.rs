//! Git 账号 DTO。
//!
//! 用户在本机配置的各 git 提交身份（多台电脑可能使用不同邮箱）。
//! 周报「只看我的提交」按启用账号的邮箱匹配提交作者。

use serde::{Deserialize, Serialize};

/// Git 账户（对应 `git_accounts` 表）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitAccount {
    #[serde(default)]
    pub id: String,
    /// 账户显示名（如 "公司电脑"）
    #[serde(default)]
    pub name: String,
    /// Git 提交邮箱，周报按此匹配作者
    #[serde(default)]
    pub email: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sort: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_account_deserializes_from_partial_json() {
        let account: GitAccount =
            serde_json::from_str(r#"{"name": "公司电脑", "email": "me@company.com"}"#)
                .expect("partial json should deserialize");
        assert_eq!(account.name, "公司电脑");
        assert!(account.enabled, "enabled should default to true");
        assert_eq!(account.sort, 0);
        assert!(account.id.is_empty());
    }
}

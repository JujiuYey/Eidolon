use serde::{Deserialize, Serialize};

/// 周报 AI 润色使用独立默认模型键，不复用聊天或请求体生成的模型
pub const WEEKLY_REPORT_POLISH_MODEL_KEY: &str = "weekly_report_polish";

/// 作者过滤方式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeeklyReportAuthorMode {
    /// 按各仓库 `git config user.email` 过滤（只看自己）
    #[default]
    Auto,
    /// 按请求中指定的作者过滤
    Custom,
    /// 不过滤，包含全部作者
    All,
}

impl WeeklyReportAuthorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            WeeklyReportAuthorMode::Auto => "auto",
            WeeklyReportAuthorMode::Custom => "custom",
            WeeklyReportAuthorMode::All => "all",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(WeeklyReportAuthorMode::Auto),
            "custom" => Ok(WeeklyReportAuthorMode::Custom),
            "all" => Ok(WeeklyReportAuthorMode::All),
            other => Err(format!("不支持的作者过滤方式: {other}")),
        }
    }
}

/// 参与周报统计的本地 git 仓库
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyReportRepo {
    #[serde(default)]
    pub id: String,

    /// 仓库绝对路径
    #[serde(default)]
    pub path: String,

    /// 展示名，默认取目录名
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub sort: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// 单条提交记录（周报聚合的原始素材）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyReportCommit {
    /// 完整哈希，跨引用去重的键
    #[serde(default)]
    pub hash: String,

    #[serde(default)]
    pub short_hash: String,

    /// 提交主题（首行）
    #[serde(default)]
    pub subject: String,

    #[serde(default)]
    pub author_name: String,

    /// 作者时间，Unix 毫秒
    #[serde(default)]
    pub timestamp: i64,
}

/// 单个仓库的提交拉取结果。拉取失败不中断整体，错误放在 `error` 里
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoCommitLog {
    #[serde(default)]
    pub repo_id: String,

    #[serde(default)]
    pub repo_name: String,

    #[serde(default)]
    pub commits: Vec<WeeklyReportCommit>,

    #[serde(default)]
    pub error: Option<String>,
}

/// 提交拉取请求。区间为 Unix 毫秒（含两端）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchCommitsRequest {
    /// 要拉取的仓库 ID；为空表示全部仓库
    #[serde(default)]
    pub repo_ids: Vec<String>,

    pub since_ms: i64,

    pub until_ms: i64,

    #[serde(default)]
    pub author_mode: WeeklyReportAuthorMode,

    /// `author_mode = custom` 时生效，匹配作者姓名或邮箱
    #[serde(default)]
    pub author: String,
}

/// 保存的周报历史
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyReportEntry {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub title: String,

    /// 统计区间起点，Unix 毫秒
    #[serde(default)]
    pub range_start: i64,

    /// 统计区间终点，Unix 毫秒
    #[serde(default)]
    pub range_end: i64,

    /// Markdown 正文
    #[serde(default)]
    pub content: String,

    #[serde(default)]
    pub repo_count: i64,

    #[serde(default)]
    pub commit_count: i64,

    #[serde(default)]
    pub created_at: i64,

    #[serde(default)]
    pub updated_at: i64,
}

/// AI 润色请求。草稿是基于提交记录聚合好的 Markdown
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolishWeeklyReportInput {
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub draft: String,
}

/// AI 润色结果
#[derive(Debug, Clone, Default, Serialize)]
pub struct PolishWeeklyReportResult {
    pub content: String,
    pub model_label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_mode_round_trips_through_string() {
        for mode in [
            WeeklyReportAuthorMode::Auto,
            WeeklyReportAuthorMode::Custom,
            WeeklyReportAuthorMode::All,
        ] {
            assert_eq!(
                WeeklyReportAuthorMode::parse(mode.as_str()).expect("parse"),
                mode
            );
        }
        assert!(WeeklyReportAuthorMode::parse("mine").is_err());
    }

    #[test]
    fn author_mode_defaults_to_auto() {
        assert_eq!(WeeklyReportAuthorMode::default(), WeeklyReportAuthorMode::Auto);
    }
}

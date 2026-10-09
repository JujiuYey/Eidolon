//! 禅道 REST API v1 客户端。
//!
//! 负责：登录 token 的获取与内存缓存（失效自动重取一次）、执行/任务/Bug 的
//! 分页拉取、"我的"数据过滤。禅道列表接口会忽略查询参数，过滤一律在客户端做。
//!
//! 接口实测结论（2026-10，禅道 22.4）：
//! - `POST /api.php/v1/tokens` 换 token，请求头 `Token: <token>` 携带
//! - `GET /api.php/v1/executions`、`/executions/{id}/tasks|bugs` 均为
//!   `{page, total, limit, <资源>}` 分页包装，每页上限 100
//! - v1 没有"我的任务"用户维度路由，需按执行展开后过滤 `assignedTo`

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::future::join;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::db::api_client::ApiClientDatabase;
use crate::db::repositories::zentao_account_repo::ZentaoAccountRepository;
use crate::models::zentao::{
    filter_my_bugs, filter_my_tasks, ZentaoAccount, ZentaoApiBug, ZentaoApiExecution,
    ZentaoApiTask, ZentaoBug, ZentaoMyWork, ZentaoTask, ZentaoUser,
    ACTIVE_EXECUTION_STATUSES,
};

const API_PREFIX: &str = "/api.php/v1";
/// 单请求超时
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// 每页条数（禅道 v1 上限 100）
const PAGE_LIMIT: u64 = 100;
/// 单接口最大翻页数，防止异常 total 导致死循环
const MAX_PAGES: u64 = 20;
/// token 内存缓存时长；禅道默认 24 小时，取一半保守值
const TOKEN_TTL: Duration = Duration::from_secs(12 * 3600);

#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    #[serde(default)]
    token: String,
}

#[derive(Debug, Deserialize)]
struct ExecutionPage {
    #[serde(default)]
    total: i64,
    #[serde(default)]
    executions: Vec<ZentaoApiExecution>,
}

#[derive(Debug, Deserialize)]
struct TaskPage {
    #[serde(default)]
    total: i64,
    #[serde(default)]
    tasks: Vec<ZentaoApiTask>,
}

#[derive(Debug, Deserialize)]
struct BugPage {
    #[serde(default)]
    total: i64,
    #[serde(default)]
    bugs: Vec<ZentaoApiBug>,
}

struct CachedToken {
    token: String,
    fetched_at: Instant,
}

/// 禅道客户端：共享 HTTP 客户端 + 按账户缓存的登录 token
pub struct ZentaoClient {
    http: reqwest::Client,
    tokens: Mutex<HashMap<String, CachedToken>>,
}

impl ZentaoClient {
    pub fn new() -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            // 站点根地址可能 302 到子路径（如 /zentao），允许有限跟随
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))?;

        Ok(Self {
            http,
            tokens: Mutex::new(HashMap::new()),
        })
    }

    /// 测试站点连通性与登录凭据
    pub async fn test_connection(
        &self,
        base_url: &str,
        login_account: &str,
        password: &str,
    ) -> Result<(), String> {
        let base = normalize_base_url(base_url)?;
        let probe = ZentaoAccount {
            account: login_account.trim().to_string(),
            password: password.to_string(),
            ..Default::default()
        };
        self.request_token(&base, &probe).await.map(|_| ())
    }

    /// 拉取"我的工作台"：当前账户名下的进行中任务与未关闭 Bug。
    ///
    /// 只展开活跃执行（wait/doing）；单个执行失败不阻断整体，记入 warnings。
    pub async fn fetch_my_work(
        &self,
        database: &ApiClientDatabase,
        account_id: &str,
    ) -> Result<ZentaoMyWork, String> {
        let account = ZentaoAccountRepository::new(database)
            .get(account_id)?
            .ok_or_else(|| format!("未找到 id 为 {account_id} 的禅道账户"))?;
        if !account.enabled {
            return Err(format!("禅道账户「{}」已停用", account.name));
        }
        let base = normalize_base_url(&account.base_url)?;

        let executions = self.fetch_executions(&base, &account).await?;
        let execution_names: HashMap<i64, String> = executions
            .iter()
            .map(|execution| (execution.id, execution.name.clone()))
            .collect();

        let mut warnings = Vec::new();
        let mut my_tasks = Vec::new();
        let mut my_bugs = Vec::new();

        for execution in executions
            .iter()
            .filter(|execution| ACTIVE_EXECUTION_STATUSES.contains(&execution.status.as_str()))
        {
            let tasks_path = format!("/executions/{}/tasks", execution.id);
            let bugs_path = format!("/executions/{}/bugs", execution.id);
            let task_future = self.fetch_all_pages(
                &base,
                &account,
                &tasks_path,
                |page: TaskPage| (page.total, page.tasks),
            );
            let bug_future = self.fetch_all_pages(
                &base,
                &account,
                &bugs_path,
                |page: BugPage| (page.total, page.bugs),
            );

            let (task_result, bug_result) = join(task_future, bug_future).await;

            match (task_result, bug_result) {
                (Ok(tasks), Ok(bugs)) => {
                    my_tasks.extend(filter_my_tasks(&tasks, &account.account));
                    my_bugs.extend(filter_my_bugs(&bugs, &account.account));
                }
                (Err(error), _) | (_, Err(error)) => {
                    warnings.push(format!(
                        "拉取执行「{}」失败: {error}",
                        execution.name
                    ));
                }
            }
        }

        let tasks: Vec<ZentaoTask> = my_tasks
            .into_iter()
            .map(|task| ZentaoTask {
                id: task.id,
                name: task.name,
                pri: task.pri,
                status: task.status,
                assigned_to: ZentaoUser::from_api(&task.assigned_to),
                execution: task.execution,
                execution_name: execution_names
                    .get(&task.execution)
                    .cloned()
                    .unwrap_or_default(),
                deadline: task.deadline,
                estimate: task.estimate,
                left: task.left,
                consumed: task.consumed,
                story_title: task.story_title,
                task_type: task.task_type,
            })
            .collect();

        let bugs: Vec<ZentaoBug> = my_bugs
            .into_iter()
            .map(|bug| ZentaoBug {
                id: bug.id,
                title: bug.title,
                severity: bug.severity,
                pri: bug.pri,
                status: bug.status,
                assigned_to: ZentaoUser::from_api(&bug.assigned_to),
                resolution: bug.resolution,
                opened_by: ZentaoUser::from_api(&bug.opened_by),
                opened_date: bug.opened_date,
                deadline: bug.deadline,
                execution: bug.execution,
                execution_name: execution_names
                    .get(&bug.execution)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect();

        Ok(ZentaoMyWork {
            account_id: account_id.to_string(),
            tasks,
            bugs,
            warnings,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn fetch_executions(
        &self,
        base: &str,
        account: &ZentaoAccount,
    ) -> Result<Vec<ZentaoApiExecution>, String> {
        self.fetch_all_pages(base, account, "/executions", |page: ExecutionPage| {
            (page.total, page.executions)
        })
        .await
    }

    /// 分页拉取一个列表资源，直到取满 total 或到达翻页上限
    async fn fetch_all_pages<P, T, F>(
        &self,
        base: &str,
        account: &ZentaoAccount,
        path: &str,
        mut extract: F,
    ) -> Result<Vec<T>, String>
    where
        P: DeserializeOwned,
        F: FnMut(P) -> (i64, Vec<T>),
    {
        let mut items = Vec::new();
        // 首页返回前不知道 total，先用哨兵值保证至少请求一页
        let mut total = i64::MAX;
        let mut page = 1u64;

        while (items.len() as i64) < total && page <= MAX_PAGES {
            let separator = if path.contains('?') { '&' } else { '?' };
            let resource_path = format!("{path}{separator}page={page}&limit={PAGE_LIMIT}");
            let page_data: P = self.get_with_retry(base, &resource_path, account).await?;
            let (page_total, mut page_items) = extract(page_data);
            total = page_total;

            let fetched = page_items.len();
            items.append(&mut page_items);

            if fetched == 0 {
                break;
            }
            page += 1;
        }

        Ok(items)
    }

    /// 带缓存的 GET：token 失效（401）时强制重取并重试一次
    async fn get_with_retry<T: DeserializeOwned>(
        &self,
        base: &str,
        path: &str,
        account: &ZentaoAccount,
    ) -> Result<T, String> {
        let token = self.cached_token(base, account).await?;
        let url = format!("{base}{API_PREFIX}{path}");

        let response = self
            .http
            .get(&url)
            .header("Token", &token)
            .send()
            .await
            .map_err(|error| send_error(&format!("禅道请求失败 ({path})"), &error))?;

        let response = if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            let fresh = self.request_token(base, account).await?;
            self.http
                .get(&url)
                .header("Token", &fresh)
                .send()
                .await
                .map_err(|error| send_error(&format!("禅道请求失败 ({path})"), &error))?
        } else {
            response
        };

        self.parse_json(response, path).await
    }

    /// 读取缓存中的 token，过期则重新获取
    async fn cached_token(&self, base: &str, account: &ZentaoAccount) -> Result<String, String> {
        if let Ok(guard) = self.tokens.lock() {
            if let Some(cached) = guard.get(&account.id) {
                if cached.fetched_at.elapsed() < TOKEN_TTL {
                    return Ok(cached.token.clone());
                }
            }
        }

        self.request_token(base, account).await
    }

    /// 登录换取新 token 并写入缓存（无 id 的探测账户不缓存）
    async fn request_token(&self, base: &str, account: &ZentaoAccount) -> Result<String, String> {
        let url = format!("{base}{API_PREFIX}/tokens");

        let response = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "account": account.account,
                "password": account.password,
            }))
            .send()
            .await
            .map_err(|error| send_error("禅道登录请求失败", &error))?;

        let payload: TokenResponse = self.parse_json(response, "/tokens").await?;
        if payload.token.is_empty() {
            return Err("禅道未返回有效 token，请检查登录凭据".to_string());
        }

        if !account.id.is_empty() {
            if let Ok(mut guard) = self.tokens.lock() {
                guard.insert(
                    account.id.clone(),
                    CachedToken {
                        token: payload.token.clone(),
                        fetched_at: Instant::now(),
                    },
                );
            }
        }

        Ok(payload.token)
    }

    async fn parse_json<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        path: &str,
    ) -> Result<T, String> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("读取禅道响应失败 ({path}): {error}"))?;

        if !status.is_success() {
            return Err(format!(
                "禅道接口 {path} 返回 {status}: {}",
                truncate(&body, 200)
            ));
        }

        serde_json::from_str(&body).map_err(|error| {
            format!(
                "解析禅道响应失败 ({path}): {error}: {}",
                truncate(&body, 200)
            )
        })
    }
}

/// 连接类失败时补充平台指引：macOS 对第三方应用强制"本地网络"权限，
/// 未授权时访问内网 IP 会直接连接失败（表现为 error sending request）
fn send_error(prefix: &str, error: &reqwest::Error) -> String {
    let message = format!("{prefix}: {error}");
    if error.is_connect() {
        #[cfg(target_os = "macos")]
        return format!(
            "{message}。若为内网地址，请在 系统设置 → 隐私与安全性 → 本地网络 中允许本应用"
        );
    }
    message
}

/// 规范化站点根地址：去尾部斜杠、要求 http(s) 前缀
pub fn normalize_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("禅道站点地址不能为空".to_string());
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("禅道站点地址需要以 http:// 或 https:// 开头".to_string());
    }
    Ok(trimmed.to_string())
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_base_url_trims_trailing_slash() {
        assert_eq!(
            normalize_base_url("http://192.168.10.209/").unwrap(),
            "http://192.168.10.209"
        );
        assert_eq!(
            normalize_base_url("  https://zentao.example.com/zentao/  ").unwrap(),
            "https://zentao.example.com/zentao"
        );
    }

    #[test]
    fn normalize_base_url_rejects_invalid_input() {
        assert!(normalize_base_url("").is_err());
        assert!(normalize_base_url("192.168.10.209").is_err());
        assert!(normalize_base_url("ftp://192.168.10.209").is_err());
    }

    #[test]
    fn truncate_limits_output_length() {
        assert_eq!(truncate("短文本", 10), "短文本");
        let long = "a".repeat(300);
        let truncated = truncate(&long, 200);
        assert!(truncated.ends_with('…'));
        assert!(truncated.chars().count() <= 201);
    }
}

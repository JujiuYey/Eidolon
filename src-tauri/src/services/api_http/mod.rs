//! 接口请求工具的 HTTP 服务层。
//!
//! 模块职责按"准备 →执行 →响应"拆分：
//!
//! - [`request_prep`] — 变量解析、URL 构造、Header / Query / Body 编码，纯函数
//! - [`client`] — reqwest 客户端、执行登记、按执行 ID 关联取消信号
//! - [`response`] — 流式读取、状态判定、二进制识别、结果对象构造

mod client;
mod request_prep;
mod response;

pub use client::{new_execution_id, ApiHttpClient, ExecutionRegistry};
pub use request_prep::{prepare_request, PrepareError, PreparedRequest, VariableScope};
pub use response::{collect_headers, is_binary_response};

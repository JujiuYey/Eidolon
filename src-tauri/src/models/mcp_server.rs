use serde::{Deserialize, Serialize};

/// 对外 MCP 服务默认端口
pub const DEFAULT_MCP_SERVER_PORT: u16 = 8765;

/// 对外 MCP 服务的设置（mcp_server_settings.json）。
/// 与 MCP 客户端配置（mcp_services.json，Eidolon 连别人的服务）相互独立。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerSettings {
    /// 是否启用（启用时随应用启动，设置变更后重启）
    #[serde(default)]
    pub enabled: bool,

    /// 监听端口，仅绑定 127.0.0.1；被占用时自动向后探测
    #[serde(default = "default_port")]
    pub port: u16,

    /// 访问令牌；客户端通过 `Authorization: Bearer <token>` 携带
    #[serde(default)]
    pub token: String,
}

impl Default for McpServerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_MCP_SERVER_PORT,
            token: String::new(),
        }
    }
}

/// 返回给前端的 MCP 服务运行信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerInfo {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub port: u16,

    #[serde(default)]
    pub token: String,

    /// 服务是否正在运行
    #[serde(default)]
    pub running: bool,

    /// 实际监听端口（运行中才有值；配置端口被占用时与 port 不同）
    #[serde(default)]
    pub active_port: Option<u16>,
}

fn default_port() -> u16 {
    DEFAULT_MCP_SERVER_PORT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let settings = McpServerSettings::default();
        assert!(!settings.enabled, "默认不启用对外服务");
        assert_eq!(settings.port, 8765);
        assert!(settings.token.is_empty(), "首次启动时由 load_or_init 生成");
    }

    #[test]
    fn deserializes_partial_json() {
        let settings: McpServerSettings =
            serde_json::from_str("{\"enabled\":true}").expect("parse");
        assert!(settings.enabled);
        assert_eq!(settings.port, 8765);
    }
}

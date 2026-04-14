use std::cell::RefCell;
use std::collections::HashMap;

use nanoid::nanoid;

use crate::db::local_store::LocalJsonStore;
use crate::models::mcp_service::{McpService, McpTransportType};

const SERVICES_FILENAME: &str = "mcp_services";

pub struct McpServiceRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, McpService>>,
}

impl<'a> McpServiceRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let cache = store.read(SERVICES_FILENAME).unwrap_or_default();

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<McpService>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then(left.id.cmp(&right.id))
        });
        Ok(results)
    }

    pub fn upsert(&self, service: &McpService) -> Result<String, String> {
        let mut normalized = normalize_service(service)?;
        if normalized.id.is_empty() {
            normalized.id = format!("mcp_{}", nanoid!(10));
        }

        let service_id = normalized.id.clone();
        self.cache
            .borrow_mut()
            .insert(service_id.clone(), normalized);
        self.store.write(SERVICES_FILENAME, &*self.cache.borrow())?;

        Ok(service_id)
    }

    pub fn delete(&self, service_id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(service_id).is_none() {
            return Err(format!("未找到 id 为 {} 的 MCP 服务", service_id));
        }

        self.store.write(SERVICES_FILENAME, &*self.cache.borrow())?;
        Ok(service_id.to_string())
    }
}

fn normalize_service(service: &McpService) -> Result<McpService, String> {
    let name = service.name.trim().to_string();
    if name.is_empty() {
        return Err("MCP 服务名称不能为空".to_string());
    }

    match service.transport_type {
        McpTransportType::Stdio => {
            if service.command.trim().is_empty() {
                return Err("STDIO 类型的命令不能为空".to_string());
            }
        }
        McpTransportType::StreamableHttp => {
            if service.url.trim().is_empty() {
                return Err("Streamable HTTP 地址不能为空".to_string());
            }
        }
    }

    Ok(McpService {
        id: service.id.trim().to_string(),
        name,
        description: service.description.trim().to_string(),
        enabled: service.enabled,
        transport_type: service.transport_type.clone(),
        command: service.command.trim().to_string(),
        args: service.args.trim().to_string(),
        env: service.env.trim().to_string(),
        url: service.url.trim().to_string(),
        long_running: service.long_running,
        timeout_seconds: service.timeout_seconds.max(1),
        discovery: service.discovery.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::McpServiceRepository;
    use crate::db::local_store::LocalJsonStore;
    use crate::models::mcp_service::{
        McpDiscoveredTool, McpService, McpServiceDiscovery, McpTransportType,
    };

    #[test]
    fn upsert_generates_id_and_persists_discovery_snapshot() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = McpServiceRepository::new(&store);

        let service = McpService {
            name: "filesystem".to_string(),
            description: "Read local files".to_string(),
            transport_type: McpTransportType::Stdio,
            command: "npx".to_string(),
            args: "@modelcontextprotocol/server-filesystem\n/Users/demo".to_string(),
            discovery: Some(McpServiceDiscovery {
                server_name: "filesystem".to_string(),
                tools: vec![McpDiscoveredTool {
                    name: "read_file".to_string(),
                    description: "Read a file".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };

        let service_id = repository
            .upsert(&service)
            .expect("service should be persisted");

        assert!(service_id.starts_with("mcp_"));

        let persisted_path = temp_dir.path().join("mcp_services.json");
        assert!(persisted_path.exists());

        let content = fs::read_to_string(&persisted_path)
            .expect("persisted mcp services should be readable");
        assert!(content.contains("\"server_name\": \"filesystem\""));
        assert!(content.contains("\"read_file\""));
    }

    #[test]
    fn upsert_requires_transport_specific_fields() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let store =
            LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
        let repository = McpServiceRepository::new(&store);

        let stdio_error = repository
            .upsert(&McpService {
                name: "bad-stdio".to_string(),
                transport_type: McpTransportType::Stdio,
                ..Default::default()
            })
            .expect_err("stdio service without command should fail");
        assert!(stdio_error.contains("命令不能为空"));

        let http_error = repository
            .upsert(&McpService {
                name: "bad-http".to_string(),
                transport_type: McpTransportType::StreamableHttp,
                ..Default::default()
            })
            .expect_err("http service without url should fail");
        assert!(http_error.contains("地址不能为空"));
    }
}

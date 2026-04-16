use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub routes_file: String,
    pub policies_dir: String,
    pub forwarding: ForwardingConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub audit: AuditConfig,
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_body_limit")]
    pub request_body_limit_bytes: usize,
}

fn default_body_limit() -> usize {
    1_048_576
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForwardingConfig {
    #[serde(default = "default_timeout")]
    #[allow(dead_code)]
    pub default_timeout_ms: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

fn default_timeout() -> u64 {
    5000
}

fn default_user_agent() -> String {
    "GuardRail/0.1.0".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "json".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

fn default_write_timeout_ms() -> u64 {
    250
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    pub token: String,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_yaml::from_str(&contents)?;

        // Environment variable overrides
        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__HOST") {
            config.server.host = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__PORT") {
            config.server.port = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_LOGGING__LEVEL") {
            config.logging.level = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATABASE__URL") {
            config.database.url = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATABASE__MAX_CONNECTIONS") {
            config.database.max_connections = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS") {
            config.audit.write_timeout_ms = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_ADMIN__TOKEN") {
            config.admin.token = val;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_valid_config() {
        let yaml = r#"
server:
  host: "127.0.0.1"
  port: 9090
  request_body_limit_bytes: 512000
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding:
  default_timeout_ms: 3000
  user_agent: "TestAgent/1.0"
logging:
  level: "debug"
  format: "pretty"
database:
  url: "postgres://test:test@localhost:5432/test"
  max_connections: 5
audit:
  write_timeout_ms: 100
admin:
  token: "test-token"
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.request_body_limit_bytes, 512000);
        assert_eq!(config.forwarding.default_timeout_ms, 3000);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_load_config_with_defaults() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 8080
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
database:
  url: "postgres://test:test@localhost:5432/test"
audit: {}
admin:
  token: "default-admin"
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.server.request_body_limit_bytes, 1_048_576);
        assert_eq!(config.forwarding.default_timeout_ms, 5000);
        assert_eq!(config.forwarding.user_agent, "GuardRail/0.1.0");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "json");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.audit.write_timeout_ms, 250);
    }

    #[test]
    fn test_load_invalid_yaml_errors() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"not: [valid: yaml: {{").unwrap();

        let result = AppConfig::load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_with_stage2_sections() {
        let yaml = r#"
server:
  host: "127.0.0.1"
  port: 9090
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
  max_connections: 12
audit:
  write_timeout_ms: 250
admin:
  token: "stage2-admin-token"
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(
            config.database.url,
            "postgres://guardrail:secret@localhost:5432/guardrail"
        );
        assert_eq!(config.database.max_connections, 12);
        assert_eq!(config.audit.write_timeout_ms, 250);
        assert_eq!(config.admin.token, "stage2-admin-token");
    }

    #[test]
    fn test_stage2_env_overrides() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 8080
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
database:
  url: "postgres://guardrail:secret@localhost:5432/guardrail"
audit: {}
admin:
  token: "from-config"
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        struct EnvGuard;

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    std::env::remove_var("GUARDRAIL_DATABASE__URL");
                    std::env::remove_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS");
                    std::env::remove_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS");
                    std::env::remove_var("GUARDRAIL_ADMIN__TOKEN");
                }
            }
        }

        let _guard = EnvGuard;

        unsafe {
            std::env::set_var(
                "GUARDRAIL_DATABASE__URL",
                "postgres://override:secret@localhost:5432/guardrail",
            );
            std::env::set_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS", "20");
            std::env::set_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS", "400");
            std::env::set_var("GUARDRAIL_ADMIN__TOKEN", "from-env");
        }

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(
            config.database.url,
            "postgres://override:secret@localhost:5432/guardrail"
        );
        assert_eq!(config.database.max_connections, 20);
        assert_eq!(config.audit.write_timeout_ms, 400);
        assert_eq!(config.admin.token, "from-env");
    }
}

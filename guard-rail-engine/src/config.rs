use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEnvironment {
    #[default]
    Development,
    Production,
}

impl std::fmt::Display for RuntimeEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeEnvironment::Development => write!(f, "development"),
            RuntimeEnvironment::Production => write!(f, "production"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AdminServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AppConfig {
    #[serde(default)]
    pub environment: RuntimeEnvironment,
    pub server: ServerConfig,
    pub routes_file: String,
    pub policies_dir: String,
    pub forwarding: ForwardingConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub audit: AuditConfig,
    pub admin: AdminConfig,
    #[serde(default)]
    pub admin_server: Option<AdminServerConfig>,
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub replay: ReplayConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub shutdown: ShutdownConfig,
    #[serde(default)]
    pub data_ops: DataOpsConfig,
    #[serde(flatten, default)]
    _extra: serde_yaml::Value,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct TenantAuthConfig {
    #[serde(default = "default_authorization_header")]
    pub header_name: String,
}

#[allow(dead_code)]
fn default_authorization_header() -> String {
    "authorization".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    #[serde(default = "default_burst")]
    pub burst: u32,
}

fn default_requests_per_minute() -> u32 {
    120
}

fn default_burst() -> u32 {
    30
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayConfig {
    #[serde(default = "default_replay_enabled")]
    pub enabled: bool,
    #[serde(default = "default_capture_request_headers")]
    pub capture_request_headers: Vec<String>,
    #[serde(default = "default_capture_response_headers")]
    pub capture_response_headers: Vec<String>,
    #[serde(default = "default_max_response_body_bytes")]
    pub max_response_body_bytes: usize,
}

fn default_replay_enabled() -> bool {
    true
}

fn default_capture_request_headers() -> Vec<String> {
    vec![
        "content-type".into(),
        "accept".into(),
        "x-request-id".into(),
    ]
}

fn default_capture_response_headers() -> Vec<String> {
    vec!["content-type".into(), "x-upstream-version".into()]
}

fn default_max_response_body_bytes() -> usize {
    65_536
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_trace_header_name")]
    pub trace_header_name: String,
    #[serde(default = "default_readiness_probe_timeout_ms")]
    pub readiness_probe_timeout_ms: u64,
}

fn default_service_name() -> String {
    "guard-rail-engine".to_string()
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_trace_header_name() -> String {
    "traceparent".to_string()
}

fn default_readiness_probe_timeout_ms() -> u64 {
    250
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            metrics_enabled: default_metrics_enabled(),
            metrics_path: default_metrics_path(),
            trace_header_name: default_trace_header_name(),
            readiness_probe_timeout_ms: default_readiness_probe_timeout_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShutdownConfig {
    #[serde(default = "default_grace_period_ms")]
    pub grace_period_ms: u64,
    #[serde(default = "default_drain_poll_interval_ms")]
    pub drain_poll_interval_ms: u64,
}

fn default_grace_period_ms() -> u64 {
    15_000
}

fn default_drain_poll_interval_ms() -> u64 {
    50
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            grace_period_ms: default_grace_period_ms(),
            drain_poll_interval_ms: default_drain_poll_interval_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataOpsConfig {
    #[serde(default = "default_audit_retention_days")]
    pub audit_retention_days: u32,
    #[serde(default = "default_artifact_retention_days")]
    pub artifact_retention_days: u32,
    #[serde(default = "default_replay_run_retention_days")]
    pub replay_run_retention_days: u32,
    #[serde(default = "default_orphan_snapshot_retention_days")]
    pub orphan_snapshot_retention_days: u32,
    #[serde(default = "default_cleanup_batch_size")]
    pub cleanup_batch_size: u32,
}

fn default_audit_retention_days() -> u32 {
    180
}
fn default_artifact_retention_days() -> u32 {
    30
}
fn default_replay_run_retention_days() -> u32 {
    30
}
fn default_orphan_snapshot_retention_days() -> u32 {
    30
}
fn default_cleanup_batch_size() -> u32 {
    1000
}

impl Default for DataOpsConfig {
    fn default() -> Self {
        Self {
            audit_retention_days: default_audit_retention_days(),
            artifact_retention_days: default_artifact_retention_days(),
            replay_run_retention_days: default_replay_run_retention_days(),
            orphan_snapshot_retention_days: default_orphan_snapshot_retention_days(),
            cleanup_batch_size: default_cleanup_batch_size(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_yaml::from_str(&contents)?;

        if config._extra.get("tenant_auth").is_some() {
            return Err("tenant_auth is no longer supported in config".into());
        }

        // Environment variable overrides
        if let Ok(val) = std::env::var("GUARDRAIL_ENVIRONMENT") {
            config.environment = match val.trim().to_ascii_lowercase().as_str() {
                "development" => RuntimeEnvironment::Development,
                "production" => RuntimeEnvironment::Production,
                other => {
                    return Err(
                        format!(
                            "invalid GUARDRAIL_ENVIRONMENT value '{}'; expected development or production",
                            other
                        )
                        .into(),
                    );
                }
            };
        }
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
        if let Ok(val) = std::env::var("GUARDRAIL_RATE_LIMIT__REQUESTS_PER_MINUTE") {
            config.rate_limit.requests_per_minute = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_RATE_LIMIT__BURST") {
            config.rate_limit.burst = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_REPLAY__ENABLED") {
            config.replay.enabled = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_OBSERVABILITY__SERVICE_NAME") {
            config.observability.service_name = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_OBSERVABILITY__METRICS_ENABLED") {
            config.observability.metrics_enabled = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_OBSERVABILITY__METRICS_PATH") {
            config.observability.metrics_path = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_OBSERVABILITY__TRACE_HEADER_NAME") {
            config.observability.trace_header_name = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_OBSERVABILITY__READINESS_PROBE_TIMEOUT_MS") {
            config.observability.readiness_probe_timeout_ms = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_SHUTDOWN__GRACE_PERIOD_MS") {
            config.shutdown.grace_period_ms = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_SHUTDOWN__DRAIN_POLL_INTERVAL_MS") {
            config.shutdown.drain_poll_interval_ms = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATA_OPS__AUDIT_RETENTION_DAYS") {
            config.data_ops.audit_retention_days = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATA_OPS__ARTIFACT_RETENTION_DAYS") {
            config.data_ops.artifact_retention_days = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATA_OPS__REPLAY_RUN_RETENTION_DAYS") {
            config.data_ops.replay_run_retention_days = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATA_OPS__ORPHAN_SNAPSHOT_RETENTION_DAYS") {
            config.data_ops.orphan_snapshot_retention_days = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_DATA_OPS__CLEANUP_BATCH_SIZE") {
            config.data_ops.cleanup_batch_size = val.parse()?;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("config test env lock poisoned")
    }

    fn clear_env_vars() {
        unsafe {
            std::env::remove_var("GUARDRAIL_ENVIRONMENT");
            std::env::remove_var("GUARDRAIL_SERVER__HOST");
            std::env::remove_var("GUARDRAIL_SERVER__PORT");
            std::env::remove_var("GUARDRAIL_LOGGING__LEVEL");
            std::env::remove_var("GUARDRAIL_DATABASE__URL");
            std::env::remove_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS");
            std::env::remove_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS");
            std::env::remove_var("GUARDRAIL_ADMIN__TOKEN");
            std::env::remove_var("GUARDRAIL_TENANT_AUTH__HEADER_NAME");
            std::env::remove_var("GUARDRAIL_RATE_LIMIT__REQUESTS_PER_MINUTE");
            std::env::remove_var("GUARDRAIL_RATE_LIMIT__BURST");
            std::env::remove_var("GUARDRAIL_REPLAY__ENABLED");
            std::env::remove_var("GUARDRAIL_OBSERVABILITY__SERVICE_NAME");
            std::env::remove_var("GUARDRAIL_OBSERVABILITY__METRICS_ENABLED");
            std::env::remove_var("GUARDRAIL_OBSERVABILITY__METRICS_PATH");
            std::env::remove_var("GUARDRAIL_OBSERVABILITY__TRACE_HEADER_NAME");
            std::env::remove_var("GUARDRAIL_OBSERVABILITY__READINESS_PROBE_TIMEOUT_MS");
            std::env::remove_var("GUARDRAIL_SHUTDOWN__GRACE_PERIOD_MS");
            std::env::remove_var("GUARDRAIL_SHUTDOWN__DRAIN_POLL_INTERVAL_MS");
            std::env::remove_var("GUARDRAIL_DATA_OPS__AUDIT_RETENTION_DAYS");
            std::env::remove_var("GUARDRAIL_DATA_OPS__ARTIFACT_RETENTION_DAYS");
            std::env::remove_var("GUARDRAIL_DATA_OPS__REPLAY_RUN_RETENTION_DAYS");
            std::env::remove_var("GUARDRAIL_DATA_OPS__ORPHAN_SNAPSHOT_RETENTION_DAYS");
            std::env::remove_var("GUARDRAIL_DATA_OPS__CLEANUP_BATCH_SIZE");
        }
    }

    struct EnvTestGuard {
        _lock: MutexGuard<'static, ()>,
    }

    impl EnvTestGuard {
        fn new() -> Self {
            let lock = env_lock();
            clear_env_vars();
            Self { _lock: lock }
        }
    }

    impl Drop for EnvTestGuard {
        fn drop(&mut self) {
            clear_env_vars();
        }
    }

    #[test]
    fn test_load_valid_config() {
        let _env = EnvTestGuard::new();
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
rate_limit:
  requests_per_minute: 120
  burst: 30
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
        let _env = EnvTestGuard::new();
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
rate_limit: {}
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
        let _env = EnvTestGuard::new();
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"not: [valid: yaml: {{").unwrap();

        let result = AppConfig::load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_with_stage3_sections() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage2-admin-token"
rate_limit:
  requests_per_minute: 120
  burst: 30
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.environment, RuntimeEnvironment::Development);
        assert_eq!(config.rate_limit.requests_per_minute, 120);
        assert_eq!(config.rate_limit.burst, 30);
    }

    #[test]
    fn test_load_config_with_stage4_replay_section() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage2-admin-token"
rate_limit: {}
replay:
  enabled: true
  capture_request_headers: ["content-type", "accept", "x-request-id"]
  capture_response_headers: ["content-type", "x-upstream-version"]
  max_response_body_bytes: 65536
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = crate::config::AppConfig::load(tmp.path()).unwrap();
        assert!(config.replay.enabled);
        assert_eq!(config.replay.capture_request_headers.len(), 3);
        assert_eq!(config.replay.max_response_body_bytes, 65536);
    }

    #[test]
    fn test_load_config_with_stage5_sections() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage-admin-token"
rate_limit: {}
replay: {}
observability:
  service_name: "guard-rail-engine"
  metrics_enabled: true
  metrics_path: "/metrics"
  trace_header_name: "traceparent"
  readiness_probe_timeout_ms: 250
shutdown:
  grace_period_ms: 15000
  drain_poll_interval_ms: 50
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = crate::config::AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.observability.metrics_path, "/metrics");
        assert_eq!(config.shutdown.grace_period_ms, 15_000);
    }

    #[test]
    fn test_load_config_with_stage5_defaults() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage-admin-token"
rate_limit: {}
replay: {}
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = crate::config::AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.observability.service_name, "guard-rail-engine");
        assert!(config.observability.metrics_enabled);
        assert_eq!(config.observability.metrics_path, "/metrics");
        assert_eq!(config.observability.trace_header_name, "traceparent");
        assert_eq!(config.observability.readiness_probe_timeout_ms, 250);
        assert_eq!(config.shutdown.grace_period_ms, 15_000);
        assert_eq!(config.shutdown.drain_poll_interval_ms, 50);
    }

    #[test]
    fn test_stage5_env_overrides() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage-admin-token"
rate_limit: {}
replay: {}
observability: {}
shutdown: {}
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        unsafe {
            std::env::set_var("GUARDRAIL_OBSERVABILITY__SERVICE_NAME", "override-service");
            std::env::set_var("GUARDRAIL_OBSERVABILITY__METRICS_ENABLED", "false");
            std::env::set_var("GUARDRAIL_OBSERVABILITY__METRICS_PATH", "/custom-metrics");
            std::env::set_var("GUARDRAIL_OBSERVABILITY__TRACE_HEADER_NAME", "x-trace-id");
            std::env::set_var("GUARDRAIL_OBSERVABILITY__READINESS_PROBE_TIMEOUT_MS", "500");
            std::env::set_var("GUARDRAIL_SHUTDOWN__GRACE_PERIOD_MS", "30000");
            std::env::set_var("GUARDRAIL_SHUTDOWN__DRAIN_POLL_INTERVAL_MS", "100");
        }

        let config = crate::config::AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.observability.service_name, "override-service");
        assert!(!config.observability.metrics_enabled);
        assert_eq!(config.observability.metrics_path, "/custom-metrics");
        assert_eq!(config.observability.trace_header_name, "x-trace-id");
        assert_eq!(config.observability.readiness_probe_timeout_ms, 500);
        assert_eq!(config.shutdown.grace_period_ms, 30_000);
        assert_eq!(config.shutdown.drain_poll_interval_ms, 100);
    }

    #[test]
    fn test_load_config_with_stage2_sections() {
        let _env = EnvTestGuard::new();
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
rate_limit:
  requests_per_minute: 120
  burst: 30
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
        let _env = EnvTestGuard::new();
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
  token: "stage-admin-token"
rate_limit: {}
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        unsafe {
            std::env::set_var(
                "GUARDRAIL_DATABASE__URL",
                "postgres://override:secret@localhost:5432/guardrail",
            );
            std::env::set_var("GUARDRAIL_DATABASE__MAX_CONNECTIONS", "20");
            std::env::set_var("GUARDRAIL_AUDIT__WRITE_TIMEOUT_MS", "400");
            std::env::set_var("GUARDRAIL_ADMIN__TOKEN", "from-env");
            std::env::set_var("GUARDRAIL_RATE_LIMIT__REQUESTS_PER_MINUTE", "200");
            std::env::set_var("GUARDRAIL_RATE_LIMIT__BURST", "50");
        }

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(
            config.database.url,
            "postgres://override:secret@localhost:5432/guardrail"
        );
        assert_eq!(config.database.max_connections, 20);
        assert_eq!(config.audit.write_timeout_ms, 400);
        assert_eq!(config.admin.token, "from-env");
        assert_eq!(config.rate_limit.requests_per_minute, 200);
        assert_eq!(config.rate_limit.burst, 50);
    }

    #[test]
    fn test_environment_env_override() {
        let _env = EnvTestGuard::new();
        let yaml = r#"
environment: development
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
  token: "stage-admin-token"
rate_limit: {}
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        unsafe {
            std::env::set_var("GUARDRAIL_ENVIRONMENT", "production");
        }

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.environment, RuntimeEnvironment::Production);
    }

    #[test]
    fn test_load_config_with_phase3_data_ops_section() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage-admin-token"
rate_limit: {}
replay: {}
data_ops:
  audit_retention_days: 180
  artifact_retention_days: 30
  replay_run_retention_days: 30
  orphan_snapshot_retention_days: 30
  cleanup_batch_size: 1000
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.data_ops.audit_retention_days, 180);
        assert_eq!(config.data_ops.artifact_retention_days, 30);
        assert_eq!(config.data_ops.replay_run_retention_days, 30);
        assert_eq!(config.data_ops.orphan_snapshot_retention_days, 30);
        assert_eq!(config.data_ops.cleanup_batch_size, 1000);
    }

    #[test]
    fn test_phase3_data_ops_env_overrides() {
        let _env = EnvTestGuard::new();
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
audit: {}
admin:
  token: "stage-admin-token"
rate_limit: {}
replay: {}
data_ops: {}
"#;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        unsafe {
            std::env::set_var("GUARDRAIL_DATA_OPS__AUDIT_RETENTION_DAYS", "365");
            std::env::set_var("GUARDRAIL_DATA_OPS__ARTIFACT_RETENTION_DAYS", "14");
            std::env::set_var("GUARDRAIL_DATA_OPS__REPLAY_RUN_RETENTION_DAYS", "14");
            std::env::set_var("GUARDRAIL_DATA_OPS__ORPHAN_SNAPSHOT_RETENTION_DAYS", "14");
            std::env::set_var("GUARDRAIL_DATA_OPS__CLEANUP_BATCH_SIZE", "250");
        }

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.data_ops.audit_retention_days, 365);
        assert_eq!(config.data_ops.artifact_retention_days, 14);
        assert_eq!(config.data_ops.replay_run_retention_days, 14);
        assert_eq!(config.data_ops.orphan_snapshot_retention_days, 14);
        assert_eq!(config.data_ops.cleanup_batch_size, 250);
    }
}

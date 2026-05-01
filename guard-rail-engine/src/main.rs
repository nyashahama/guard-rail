mod audit;
mod auth;
mod config;
mod execution;
mod logging;
mod observability;
mod policy;
mod proxy;
mod reload;
mod replay;
mod routes;
mod shutdown;
mod storage;
mod tenant;

pub use routes::RouteAuthMode;

use audit::hash::hash_string;
use clap::Parser;
use proxy::AppState;
use reqwest::Client;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tenant::cache::{TenantAuthCache, validate_route_auth_state};
use tenant::repository::TenantRepository;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[derive(Debug, Clone, clap::Subcommand, Default)]
enum Command {
    #[default]
    Serve,
    Migrate,
    Cleanup {
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
}

#[derive(Parser)]
#[command(name = "guard-rail-engine", about = "Zero-trust execution runtime")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, default_value = "./config/config.yaml", global = true)]
    config: PathBuf,
}

fn validate_startup_security(config: &config::AppConfig) -> Result<(), String> {
    if !matches!(config.environment, config::RuntimeEnvironment::Production) {
        return Ok(());
    }

    if config.database.url.trim().is_empty() {
        return Err(
            "invalid database URL for production runtime; database.url cannot be empty".to_string(),
        );
    }

    if config.audit.persistence_mode != config::AuditPersistenceMode::RequiredBeforeResponse {
        return Err(
            "invalid audit.persistence_mode for production runtime; expected required_before_response"
                .to_string(),
        );
    }

    if config.audit.write_timeout_ms == 0 {
        return Err(
            "invalid audit.write_timeout_ms for production runtime; value must be greater than 0"
                .to_string(),
        );
    }

    if config.server.request_body_limit_bytes == 0 {
        return Err(
            "invalid server.request_body_limit_bytes for production runtime; value must be greater than 0"
                .to_string(),
        );
    }

    if config.replay.enabled {
        if config.replay.max_response_body_bytes == 0 {
            return Err(
                "invalid replay.max_response_body_bytes for production runtime; value must be greater than 0"
                    .to_string(),
            );
        }

        if config.replay.redact_request_headers.is_empty()
            || config.replay.redact_response_headers.is_empty()
            || config.replay.redact_json_fields.is_empty()
            || config.replay.redaction_text.trim().is_empty()
        {
            return Err(
                "invalid replay redaction policy for production runtime; redact_request_headers, redact_response_headers, redact_json_fields, and redaction_text must all be configured"
                    .to_string(),
            );
        }
    }

    if let Some(admin_server) = &config.admin_server {
        let token = config.admin.token.trim();
        if token.is_empty() || token.eq_ignore_ascii_case("change-me") {
            return Err(
                "invalid admin token for production admin listener; configure a non-default token"
                    .to_string(),
            );
        }

        if matches!(admin_server.host.trim(), "0.0.0.0" | "::") {
            return Err(
                "invalid admin listener for production runtime; admin listener cannot bind to 0.0.0.0 or ::"
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let app_config = config::AppConfig::load(&cli.config)?;

    let command = cli.command.unwrap_or_default();

    match command {
        Command::Migrate => {
            let pool = storage::postgres::connect_pool(&app_config.database).await?;
            storage::postgres::run_migrations(&pool).await?;
            println!("Migrations applied successfully");
            return Ok(());
        }
        Command::Cleanup { apply } => {
            let pool = storage::postgres::connect_pool(&app_config.database).await?;
            storage::postgres::assert_schema_ready(&pool).await?;

            let manager =
                storage::retention::RetentionManager::new(pool, app_config.data_ops.clone());

            if apply {
                let result = manager.apply().await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let preview = manager.preview().await?;
                println!("{}", serde_json::to_string_pretty(&preview)?);
            }

            return Ok(());
        }
        Command::Serve => {
            validate_startup_security(&app_config)
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
        }
    }

    observability::tracing::init(&app_config.logging, &app_config.observability)
        .map_err(|err| -> Box<dyn std::error::Error> { err })?;

    let pool = storage::postgres::connect_pool(&app_config.database).await?;
    storage::postgres::assert_schema_ready(&pool).await?;

    tracing::info!("Loading routes from {}", app_config.routes_file);
    let route_table = routes::RouteTable::load(&PathBuf::from(&app_config.routes_file))?;
    let route_table_for_cache = route_table.clone();

    tracing::info!("Loading policies from {}", app_config.policies_dir);
    let policy_set = policy::PolicySet::load_dir(&PathBuf::from(&app_config.policies_dir))?;

    let required_policies = route_table.policy_names();
    policy_set
        .validate_references(&required_policies)
        .map_err(|e| format!("Policy validation failed: {}", e))?;

    let routes = Arc::new(RwLock::new(route_table));
    let policies = Arc::new(RwLock::new(policy_set));

    let tenant_repo = TenantRepository::new(pool.clone());
    let auth_snapshot = tenant_repo.load_auth_snapshot().await?;

    validate_route_auth_state(&route_table_for_cache, &auth_snapshot)
        .map_err(|e| format!("Tenant auth validation failed: {}", e))?;

    routes::RouteValidator::validate_upstream_security(
        &route_table_for_cache,
        app_config.environment,
    )
    .map_err(|e| format!("Upstream security validation failed: {}", e))?;

    let tenant_cache = TenantAuthCache::default();
    tenant_cache.replace(auth_snapshot).await;

    let metrics = if app_config.observability.metrics_enabled {
        match observability::metrics::Metrics::new() {
            Ok(metrics) => Some(Arc::new(metrics)),
            Err(err) => {
                tracing::error!(error = %err, "failed to initialize metrics");
                None
            }
        }
    } else {
        None
    };

    let reload_routes = Arc::clone(&routes);
    let reload_policies = Arc::clone(&policies);
    reload::start_watcher(
        PathBuf::from(&app_config.routes_file),
        PathBuf::from(&app_config.policies_dir),
        reload_routes,
        reload_policies,
        tenant_cache.clone(),
        app_config.environment,
        metrics.clone(),
    )?;

    let http_client = Client::builder()
        .user_agent(&app_config.forwarding.user_agent)
        .build()?;

    let audit_store = storage::postgres::PostgresAuditStore::new(
        pool.clone(),
        std::time::Duration::from_millis(250),
    );
    let lifecycle = shutdown::LifecycleState::new();

    let route_config_path = PathBuf::from(&app_config.routes_file);
    let policies_dir_path = PathBuf::from(&app_config.policies_dir);

    let route_config_hash = if route_config_path.exists() {
        hash_string(&std::fs::read_to_string(&route_config_path).unwrap_or_default())
    } else {
        hash_string("")
    };

    let policy_set_hash = if policies_dir_path.exists() {
        let mut combined = String::new();
        if let Ok(entries) = std::fs::read_dir(&policies_dir_path) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .is_some_and(|e| e == "yaml" || e == "yml")
                {
                    combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
                }
            }
        }
        hash_string(&combined)
    } else {
        hash_string("")
    };

    let state = AppState {
        routes,
        policies,
        http_client,
        audit_store: Some(audit_store),
        audit_persistence_mode: app_config.audit.persistence_mode,
        metrics: metrics.clone(),
        lifecycle: lifecycle.clone(),
        readiness_probe_timeout_ms: app_config.observability.readiness_probe_timeout_ms,
        trace_header_name: app_config.observability.trace_header_name.clone(),
        route_config_hash,
        policy_set_hash,
        admin_token: app_config.admin.token.clone(),
        tenant_repo,
        tenant_cache,
        rate_limiter: crate::auth::rate_limit::TenantRateLimiter::new(
            app_config.rate_limit.requests_per_minute,
            app_config.rate_limit.burst,
        ),
        replay: app_config.replay.clone(),
    };

    let main_app = proxy::build_main_router(
        state.clone(),
        app_config.server.request_body_limit_bytes,
        &app_config.observability,
    );

    let admin_app = proxy::build_admin_router(state.clone(), app_config.admin.token.clone());

    let main_addr: SocketAddr =
        format!("{}:{}", app_config.server.host, app_config.server.port).parse()?;

    tracing::info!("Guard Rail Engine starting main listener on {}", main_addr);

    let main_listener = TcpListener::bind(main_addr).await?;

    let admin_handle = if let Some(admin_config) = &app_config.admin_server {
        let admin_addr: SocketAddr =
            format!("{}:{}", admin_config.host, admin_config.port).parse()?;
        tracing::info!(
            "Guard Rail Engine starting admin listener on {}",
            admin_addr
        );
        let admin_listener = TcpListener::bind(admin_addr).await?;

        let admin_app = admin_app.into_make_service();
        Some(tokio::spawn(async move {
            axum::serve(admin_listener, admin_app).await
        }))
    } else {
        None
    };

    lifecycle.mark_ready().await;
    if let Some(metrics) = &metrics {
        metrics.set_readiness(true);
        metrics.record_shutdown_transition("ready");
    }

    let (drain_tx, drain_rx) = tokio::sync::oneshot::channel::<()>();
    let main_server = async move {
        axum::serve(
            main_listener,
            main_app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = drain_rx.await;
        })
        .await
    };

    let grace_period = std::time::Duration::from_millis(app_config.shutdown.grace_period_ms);
    tokio::pin!(main_server);

    tokio::select! {
        result = &mut main_server => {
            if let Err(err) = result {
                return Err(Box::<dyn std::error::Error>::from(err));
            }
        }
        _ = shutdown::wait_for_signal() => {
            lifecycle.begin_drain().await;
            if let Some(metrics) = &metrics {
                metrics.set_readiness(false);
                metrics.record_shutdown_transition("draining");
            }
            let _ = drain_tx.send(());

            match tokio::time::timeout(grace_period, &mut main_server).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => return Err(Box::<dyn std::error::Error>::from(err)),
                Err(_) => {
                    let inflight_requests = metrics
                        .as_ref()
                        .map(|metrics| metrics.inflight_requests())
                        .unwrap_or(0);
                    tracing::warn!(inflight_requests, "grace period expired while draining");
                }
            }
        }
    }

    if let Some(handle) = admin_handle {
        handle.abort();
    }

    lifecycle.mark_stopped().await;
    if let Some(metrics) = &metrics {
        metrics.set_readiness(false);
        metrics.record_shutdown_transition("stopped");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::io::Write;

    struct TestConfigOptions<'a> {
        environment: &'a str,
        admin_server_host: Option<&'a str>,
        admin_token: &'a str,
        database_url: &'a str,
        audit_persistence_mode: &'a str,
        audit_write_timeout_ms: u64,
        request_body_limit_bytes: usize,
        replay_enabled: bool,
        redact_request_headers: &'a str,
        redact_response_headers: &'a str,
        redact_json_fields: &'a str,
        redaction_text: &'a str,
        max_response_body_bytes: usize,
    }

    impl Default for TestConfigOptions<'_> {
        fn default() -> Self {
            Self {
                environment: "production",
                admin_server_host: Some("127.0.0.1"),
                admin_token: "super-secret-token",
                database_url: "postgres://user:pass@localhost/db",
                audit_persistence_mode: "required_before_response",
                audit_write_timeout_ms: 250,
                request_body_limit_bytes: 1_048_576,
                replay_enabled: true,
                redact_request_headers: r#"["authorization"]"#,
                redact_response_headers: r#"["set-cookie"]"#,
                redact_json_fields: r#"["token"]"#,
                redaction_text: "[REDACTED]",
                max_response_body_bytes: 65_536,
            }
        }
    }

    fn test_config(options: TestConfigOptions<'_>) -> config::AppConfig {
        let admin_server = if let Some(host) = options.admin_server_host {
            r#"
admin_server:
  host: "{host}"
  port: 8081
"#
            .replace("{host}", host)
        } else {
            String::new()
        };

        let yaml = format!(
            r#"
environment: {environment}
server:
  host: "127.0.0.1"
  port: 8080
  request_body_limit_bytes: {request_body_limit_bytes}
routes_file: "./config/routes.yaml"
policies_dir: "./config/policies"
forwarding: {{}}
logging: {{}}
database:
  url: "{database_url}"
audit:
  write_timeout_ms: {audit_write_timeout_ms}
  persistence_mode: {audit_persistence_mode}
admin:
  token: "{admin_token}"
{admin_server}
rate_limit: {{}}
replay:
  enabled: {replay_enabled}
  capture_request_headers: ["content-type"]
  capture_response_headers: ["content-type"]
  redact_request_headers: {redact_request_headers}
  redact_response_headers: {redact_response_headers}
  redact_json_fields: {redact_json_fields}
  redaction_text: "{redaction_text}"
  max_response_body_bytes: {max_response_body_bytes}
"#,
            environment = options.environment,
            request_body_limit_bytes = options.request_body_limit_bytes,
            database_url = options.database_url,
            audit_write_timeout_ms = options.audit_write_timeout_ms,
            audit_persistence_mode = options.audit_persistence_mode,
            admin_token = options.admin_token,
            admin_server = admin_server,
            replay_enabled = options.replay_enabled,
            redact_request_headers = options.redact_request_headers,
            redact_response_headers = options.redact_response_headers,
            redact_json_fields = options.redact_json_fields,
            redaction_text = options.redaction_text,
            max_response_body_bytes = options.max_response_body_bytes,
        );

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();
        config::AppConfig::load(tmp.path()).unwrap()
    }

    #[test]
    fn test_migrate_command_parses() {
        let cli = Cli::try_parse_from([
            "guard-rail-engine",
            "migrate",
            "--config",
            "./config/config.yaml",
        ])
        .unwrap();

        assert!(matches!(cli.command, Some(Command::Migrate)));
        assert_eq!(cli.config, PathBuf::from("./config/config.yaml"));
    }

    #[test]
    fn test_serve_is_default_command() {
        let cli = Cli::try_parse_from(["guard-rail-engine"]).unwrap();
        let command = cli.command.unwrap_or_default();
        assert!(matches!(command, Command::Serve));
    }

    #[test]
    fn test_validate_startup_security_production_admin_listener_rejects_default_token() {
        let cfg = test_config(TestConfigOptions {
            admin_token: "change-me",
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("invalid admin token"));
    }

    #[test]
    fn test_validate_startup_security_production_admin_listener_rejects_empty_token() {
        let cfg = test_config(TestConfigOptions {
            admin_token: "   ",
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("invalid admin token"));
    }

    #[test]
    fn test_validate_startup_security_development_allows_default_token() {
        let cfg = test_config(TestConfigOptions {
            environment: "development",
            admin_token: "change-me",
            ..Default::default()
        });
        assert!(validate_startup_security(&cfg).is_ok());
    }

    #[test]
    fn test_validate_startup_security_production_rejects_blank_database_url() {
        let cfg = test_config(TestConfigOptions {
            database_url: "   ",
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("database URL"));
    }

    #[test]
    fn test_validate_startup_security_production_rejects_non_durable_audit_mode() {
        let cfg = test_config(TestConfigOptions {
            audit_persistence_mode: "best_effort",
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("audit.persistence_mode"));
    }

    #[test]
    fn test_validate_startup_security_production_rejects_zero_audit_write_timeout() {
        let cfg = test_config(TestConfigOptions {
            audit_write_timeout_ms: 0,
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("audit.write_timeout_ms"));
    }

    #[test]
    fn test_validate_startup_security_production_rejects_zero_request_body_limit() {
        let cfg = test_config(TestConfigOptions {
            request_body_limit_bytes: 0,
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("server.request_body_limit_bytes"));
    }

    #[test]
    fn test_validate_startup_security_production_rejects_zero_replay_response_body_limit_when_enabled()
     {
        let cfg = test_config(TestConfigOptions {
            max_response_body_bytes: 0,
            ..Default::default()
        });
        let err = validate_startup_security(&cfg).unwrap_err();
        assert!(err.contains("replay.max_response_body_bytes"));
    }

    #[test]
    fn test_validate_startup_security_production_rejects_empty_replay_redaction_policy_when_enabled()
     {
        let cases = [
            TestConfigOptions {
                redact_request_headers: "[]",
                ..Default::default()
            },
            TestConfigOptions {
                redact_response_headers: "[]",
                ..Default::default()
            },
            TestConfigOptions {
                redact_json_fields: "[]",
                ..Default::default()
            },
            TestConfigOptions {
                redaction_text: "   ",
                ..Default::default()
            },
        ];

        for options in cases {
            let err = validate_startup_security(&test_config(options)).unwrap_err();
            assert!(err.contains("replay redaction policy"));
        }
    }

    #[test]
    fn test_validate_startup_security_production_rejects_wildcard_admin_listener_hosts() {
        for host in ["0.0.0.0", "::"] {
            let cfg = test_config(TestConfigOptions {
                admin_server_host: Some(host),
                ..Default::default()
            });
            let err = validate_startup_security(&cfg).unwrap_err();
            assert!(err.contains("admin listener"));
        }
    }

    #[test]
    fn test_validate_startup_security_development_allows_unsafe_local_defaults() {
        let cfg = test_config(TestConfigOptions {
            environment: "development",
            admin_server_host: Some("0.0.0.0"),
            admin_token: "change-me",
            database_url: "",
            audit_persistence_mode: "best_effort",
            audit_write_timeout_ms: 0,
            request_body_limit_bytes: 0,
            redact_request_headers: "[]",
            redact_response_headers: "[]",
            redact_json_fields: "[]",
            redaction_text: "",
            max_response_body_bytes: 0,
            ..Default::default()
        });
        assert!(validate_startup_security(&cfg).is_ok());
    }

    #[test]
    fn test_validate_startup_security_production_allows_disabled_replay_without_redaction_requirements()
     {
        let cfg = test_config(TestConfigOptions {
            replay_enabled: false,
            redact_request_headers: "[]",
            redact_response_headers: "[]",
            redact_json_fields: "[]",
            redaction_text: "",
            max_response_body_bytes: 0,
            ..Default::default()
        });
        assert!(validate_startup_security(&cfg).is_ok());
    }

    #[test]
    fn test_cleanup_command_parses_without_apply() {
        let cli = Cli::try_parse_from([
            "guard-rail-engine",
            "cleanup",
            "--config",
            "./config/config.yaml",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Cleanup { apply: false })
        ));
    }

    #[test]
    fn test_cleanup_command_parses_with_apply_flag() {
        let cli = Cli::try_parse_from([
            "guard-rail-engine",
            "cleanup",
            "--config",
            "./config/config.yaml",
            "--apply",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Cleanup { apply: true })
        ));
    }
}

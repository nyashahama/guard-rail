mod audit;
mod auth;
mod config;
mod execution;
mod logging;
mod policy;
mod proxy;
mod reload;
mod replay;
mod routes;
mod storage;
mod tenant;

use audit::hash::hash_string;
use clap::Parser;
use proxy::AppState;
use reqwest::Client;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tenant::cache::{TenantAuthCache, validate_all_routes_bound};
use tenant::repository::TenantRepository;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, clap::Subcommand, Default)]
enum Command {
    #[default]
    Serve,
    Migrate,
}

#[derive(Parser)]
#[command(name = "guard-rail-engine", about = "Zero-trust execution runtime")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, default_value = "./config/config.yaml", global = true)]
    config: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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
        Command::Serve => {}
    }

    let pool = storage::postgres::connect_pool(&app_config.database).await?;
    storage::postgres::assert_schema_ready(&pool).await?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&app_config.logging.level));

    if app_config.logging.format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .pretty()
            .init();
    }

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

    let reload_routes = Arc::clone(&routes);
    let reload_policies = Arc::clone(&policies);
    reload::start_watcher(
        PathBuf::from(&app_config.routes_file),
        PathBuf::from(&app_config.policies_dir),
        reload_routes,
        reload_policies,
    )?;

    let http_client = Client::builder()
        .user_agent(&app_config.forwarding.user_agent)
        .build()?;

    let audit_store = storage::postgres::PostgresAuditStore::new(
        pool.clone(),
        std::time::Duration::from_millis(250),
    );

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

    let tenant_repo = TenantRepository::new(pool.clone());
    let tenant_cache = TenantAuthCache::default();
    let auth_snapshot = tenant_repo.load_auth_snapshot().await?;
    validate_all_routes_bound(&route_table_for_cache, &auth_snapshot)
        .map_err(|err| format!("Tenant binding validation failed: {err}"))?;
    tenant_cache.replace(auth_snapshot).await;

    let state = AppState {
        routes,
        policies,
        http_client,
        audit_store: Some(audit_store),
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

    let app = proxy::build_router(
        state,
        app_config.admin.token.clone(),
        app_config.server.request_body_limit_bytes,
    );

    let addr: SocketAddr =
        format!("{}:{}", app_config.server.host, app_config.server.port).parse()?;

    tracing::info!("Guard Rail Engine starting on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

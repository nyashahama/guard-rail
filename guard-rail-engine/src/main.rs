mod config;
mod logging;
mod policy;
mod proxy;
mod reload;
mod routes;
mod storage;

use clap::Parser;
use proxy::AppState;
use reqwest::Client;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Serve,
    Migrate,
}

impl Default for Command {
    fn default() -> Self {
        Command::Serve
    }
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
        assert!(matches!(cli.command, Some(Command::Serve)));
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
    drop(pool);

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

    let state = AppState {
        routes,
        policies,
        http_client,
    };

    let app = axum::Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(proxy::handle_execute),
        )
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            app_config.server.request_body_limit_bytes,
        ))
        .with_state(state);

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

mod config;
mod logging;
mod policy;
mod proxy;
mod reload;
mod routes;

use clap::Parser;
use proxy::AppState;
use reqwest::Client;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "guard-rail-engine", about = "Zero-trust execution runtime")]
struct Cli {
    /// Path to config.yaml
    #[arg(short, long, default_value = "./config/config.yaml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load config
    let app_config = config::AppConfig::load(&cli.config)?;

    // Setup tracing
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

    // Validate that all route policy references exist
    let required_policies = route_table.policy_names();
    policy_set
        .validate_references(&required_policies)
        .map_err(|e| format!("Policy validation failed: {}", e))?;

    let routes = Arc::new(RwLock::new(route_table));
    let policies = Arc::new(RwLock::new(policy_set));

    // Start hot-reload watcher
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

pub mod forward;
pub mod response;

use crate::audit::hash::{hash_body, hash_string};
use crate::execution::{ExecutionRecord, ExecutionVerdict};
use crate::logging::ExecutionLog;
use crate::policy::PolicySet;
use crate::policy::engine::{Verdict, evaluate};
use crate::routes::RouteTable;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub routes: Arc<RwLock<RouteTable>>,
    pub policies: Arc<RwLock<PolicySet>>,
    pub http_client: Client,
    pub audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("routes", &"<RwLock<RouteTable>>")
            .field("policies", &"<RwLock<PolicySet>>")
            .field("http_client", &"<Client>")
            .field("audit_store", &self.audit_store.is_some())
            .field("route_config_hash", &self.route_config_hash)
            .field("policy_set_hash", &self.policy_set_hash)
            .finish()
    }
}

async fn emit_and_persist(record: ExecutionRecord, audit_store: Option<crate::storage::postgres::PostgresAuditStore>) {
    ExecutionLog::from(&record).emit();
    if let Some(store) = audit_store {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(250),
            store.insert_execution(&record),
        ).await;
    }
}

fn spawn_emit_and_persist(record: ExecutionRecord, audit_store: Option<crate::storage::postgres::PostgresAuditStore>) {
    tokio::spawn(async move {
        emit_and_persist(record, audit_store).await;
    });
}

pub async fn handle_execute(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(route_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let total_start = Instant::now();
    let execution_id = format!("GR-EXE-{}", Uuid::new_v4());
    let source_ip = addr.ip().to_string();
    let execution_started_at = Utc::now();

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let had_authorization_header = headers.contains_key("authorization");
    let request_size_bytes = body.len();
    let request_body_sha256 = hash_body(&body);

    // 1. Route lookup
    let routes = state.routes.read().await;
    let route = match routes.lookup(&route_id) {
        Some(r) => r.clone(),
        None => {
            return (axum::http::StatusCode::NOT_FOUND, "Route not found").into_response();
        }
    };
    drop(routes);

    // 2. Method check
    let method_str = method.to_string();
    if !route.methods.contains(&method_str) {
        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            execution_started_at,
            route_id: route_id.clone(),
            upstream_url: Some(route.upstream.clone()),
            method: method_str.clone(),
            source_ip: source_ip.clone(),
            content_type,
            user_agent,
            had_authorization_header,
            request_size_bytes,
            request_body_sha256,
            verdict: ExecutionVerdict::Rejected,
            rejection_reason: Some("method_not_allowed".to_string()),
            matched_policy_name: None,
            matched_rule_field: None,
            matched_rule_condition: None,
            matched_rule_severity: None,
            violation_value_hash: None,
            violation_value_preview: None,
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 0,
            latency_forward_ms: None,
            latency_total_ms: total_start.elapsed().as_millis(),
            route_config_hash: state.route_config_hash.clone(),
            policy_set_hash: state.policy_set_hash.clone(),
        };
        spawn_emit_and_persist(record, state.audit_store.clone());
        return response::method_not_allowed_response(&execution_id);
    }

    // 3. Parse body as JSON
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            let record = ExecutionRecord {
                execution_id: execution_id.clone(),
                execution_started_at,
                route_id: route_id.clone(),
                upstream_url: Some(route.upstream.clone()),
                method: method_str.clone(),
                source_ip: source_ip.clone(),
                content_type,
                user_agent,
                had_authorization_header,
                request_size_bytes,
                request_body_sha256,
                verdict: ExecutionVerdict::Rejected,
                rejection_reason: Some("invalid_json".to_string()),
                matched_policy_name: None,
                matched_rule_field: None,
                matched_rule_condition: None,
                matched_rule_severity: None,
                violation_value_hash: None,
                violation_value_preview: None,
                upstream_status: None,
                forward_error: None,
                latency_inspect_us: 0,
                latency_forward_ms: None,
                latency_total_ms: total_start.elapsed().as_millis(),
                route_config_hash: state.route_config_hash.clone(),
                policy_set_hash: state.policy_set_hash.clone(),
            };
            spawn_emit_and_persist(record, state.audit_store.clone());
            return response::reject_response(&execution_id, "Invalid JSON body");
        }
    };

    // 4. Policy evaluation
    let inspect_start = Instant::now();
    let policies = state.policies.read().await;
    let verdict = evaluate(&payload, body.len(), &route.policies, &policies);
    drop(policies);
    let latency_inspect_us = inspect_start.elapsed().as_micros();

    match verdict {
        Verdict::Block {
            policy_name,
            rule_field,
            message,
            violation_value,
        } => {
            let violation_value_hash = violation_value.as_ref().map(|v| hash_string(v));
            let violation_value_preview = violation_value
                .as_ref()
                .and_then(|v| crate::audit::hash::preview_violation_value(v));

            let record = ExecutionRecord {
                execution_id: execution_id.clone(),
                execution_started_at,
                route_id: route_id.clone(),
                upstream_url: Some(route.upstream.clone()),
                method: method_str.clone(),
                source_ip: source_ip.clone(),
                content_type,
                user_agent,
                had_authorization_header,
                request_size_bytes,
                request_body_sha256,
                verdict: ExecutionVerdict::Blocked,
                rejection_reason: None,
                matched_policy_name: Some(policy_name.clone()),
                matched_rule_field: Some(rule_field.clone()),
                matched_rule_condition: None,
                matched_rule_severity: None,
                violation_value_hash,
                violation_value_preview,
                upstream_status: None,
                forward_error: None,
                latency_inspect_us,
                latency_forward_ms: None,
                latency_total_ms: total_start.elapsed().as_millis(),
                route_config_hash: state.route_config_hash.clone(),
                policy_set_hash: state.policy_set_hash.clone(),
            };
            spawn_emit_and_persist(record, state.audit_store.clone());
            return response::block_response(&execution_id, &policy_name, &rule_field, &message);
        }
        Verdict::Allow => {}
    }

    // 5. Forward to upstream
    let forward_start = Instant::now();

    match forward::forward_request(
        &state.http_client,
        &route.upstream,
        &method_str,
        body,
        &headers,
        &execution_id,
        route.timeout_ms,
    )
    .await
    {
        Ok(result) => {
            let record = ExecutionRecord {
                execution_id: execution_id.clone(),
                execution_started_at,
                route_id: route_id.clone(),
                upstream_url: Some(route.upstream.clone()),
                method: method_str.clone(),
                source_ip: source_ip.clone(),
                content_type,
                user_agent,
                had_authorization_header,
                request_size_bytes,
                request_body_sha256,
                verdict: ExecutionVerdict::Allowed,
                rejection_reason: None,
                matched_policy_name: None,
                matched_rule_field: None,
                matched_rule_condition: None,
                matched_rule_severity: None,
                violation_value_hash: None,
                violation_value_preview: None,
                upstream_status: Some(result.status),
                forward_error: None,
                latency_inspect_us,
                latency_forward_ms: Some(forward_start.elapsed().as_millis()),
                latency_total_ms: total_start.elapsed().as_millis(),
                route_config_hash: state.route_config_hash.clone(),
                policy_set_hash: state.policy_set_hash.clone(),
            };
            spawn_emit_and_persist(record, state.audit_store.clone());
            result.response
        }
        Err(e) => {
            let record = ExecutionRecord {
                execution_id: execution_id.clone(),
                execution_started_at,
                route_id: route_id.clone(),
                upstream_url: Some(route.upstream.clone()),
                method: method_str.clone(),
                source_ip: source_ip.clone(),
                content_type,
                user_agent,
                had_authorization_header,
                request_size_bytes,
                request_body_sha256,
                verdict: ExecutionVerdict::Allowed,
                rejection_reason: None,
                matched_policy_name: None,
                matched_rule_field: None,
                matched_rule_condition: None,
                matched_rule_severity: None,
                violation_value_hash: None,
                violation_value_preview: None,
                upstream_status: None,
                forward_error: Some(e.clone()),
                latency_inspect_us,
                latency_forward_ms: Some(forward_start.elapsed().as_millis()),
                latency_total_ms: total_start.elapsed().as_millis(),
                route_config_hash: state.route_config_hash.clone(),
                policy_set_hash: state.policy_set_hash.clone(),
            };
            spawn_emit_and_persist(record, state.audit_store.clone());
            response::bad_gateway_response(&execution_id, &e)
        }
    }
}

pub fn build_router(
    state: AppState,
    admin_token: String,
    request_body_limit_bytes: usize,
) -> axum::Router {
    let audit_routes = axum::Router::new()
        .route(
            "/v1/audit/executions",
            axum::routing::get(crate::audit::api::list_executions),
        )
        .route(
            "/v1/audit/executions/{execution_id}",
            axum::routing::get(crate::audit::api::get_execution),
        )
        .route(
            "/v1/audit/integrity",
            axum::routing::get(crate::audit::api::verify_integrity),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            admin_token,
            crate::auth::middleware::require_admin_token,
        ))
        .with_state(state.clone());

    let main_router = axum::Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(handle_execute),
        )
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(audit_routes)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            request_body_limit_bytes,
        ));

    main_router.with_state(state)
}

pub fn compute_state(
    routes_path: &std::path::Path,
    policies_dir: &std::path::Path,
    http_client: Client,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
) -> AppState {
    let route_table = RouteTable::load(routes_path).unwrap();
    let policy_set = PolicySet::load_dir(policies_dir).unwrap();

    let route_config_hash = if routes_path.exists() {
        hash_string(&std::fs::read_to_string(routes_path).unwrap_or_default())
    } else {
        hash_string("")
    };

    let policy_set_hash = if policies_dir.exists() {
        let mut combined = String::new();
        if let Ok(entries) = std::fs::read_dir(policies_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map_or(false, |e| e == "yaml" || e == "yml") {
                    combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
                }
            }
        }
        hash_string(&combined)
    } else {
        hash_string("")
    };

    AppState {
        routes: Arc::new(RwLock::new(route_table)),
        policies: Arc::new(RwLock::new(policy_set)),
        http_client,
        audit_store,
        route_config_hash,
        policy_set_hash,
    }
}

pub mod forward;
pub mod response;

use crate::logging::ExecutionLog;
use crate::policy::engine::{evaluate, Verdict};
use crate::policy::PolicySet;
use crate::routes::RouteTable;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method};
use axum::response::{IntoResponse, Response};
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

    let mut log = ExecutionLog::new(
        execution_id.clone(),
        route_id.clone(),
        method.to_string(),
        source_ip,
    );

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
        log.verdict = "REJECTED".to_string();
        log.latency_total_ms = total_start.elapsed().as_millis();
        log.emit();
        return response::method_not_allowed_response(&execution_id);
    }

    // 3. Parse body as JSON
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            log.verdict = "REJECTED".to_string();
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            return response::reject_response(&execution_id, "Invalid JSON body");
        }
    };

    // 4. Policy evaluation
    let inspect_start = Instant::now();
    let policies = state.policies.read().await;
    let verdict = evaluate(&payload, body.len(), &route.policies, &policies);
    drop(policies);
    log.latency_inspect_us = inspect_start.elapsed().as_micros();

    match verdict {
        Verdict::Block {
            policy_name,
            rule_field,
            message,
            violation_value,
        } => {
            log.verdict = "BLOCKED".to_string();
            log.policy = Some(policy_name.clone());
            log.rule_field = Some(rule_field.clone());
            log.violation_value = violation_value;
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            return response::block_response(&execution_id, &policy_name, &rule_field, &message);
        }
        Verdict::Allow => {}
    }

    // 5. Forward to upstream
    log.upstream = Some(route.upstream.clone());
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
            log.verdict = "ALLOWED".to_string();
            log.upstream_status = Some(result.status);
            log.latency_forward_ms = Some(forward_start.elapsed().as_millis());
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            result.response
        }
        Err(e) => {
            log.verdict = "ALLOWED".to_string();
            log.forward_error = Some(e.clone());
            log.latency_forward_ms = Some(forward_start.elapsed().as_millis());
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            response::bad_gateway_response(&execution_id, &e)
        }
    }
}

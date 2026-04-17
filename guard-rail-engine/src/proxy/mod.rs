pub mod forward;
pub mod response;

use crate::config::ReplayConfig;
use crate::replay::snapshot;
use crate::audit::hash::{hash_body, hash_string};
use crate::auth::context::{RequestAuthContext, authenticate_tenant_request};
use crate::auth::rate_limit::TenantRateLimiter;
use crate::execution::{ExecutionRecord, ExecutionVerdict};
use crate::logging::ExecutionLog;
use crate::policy::PolicySet;
use crate::policy::engine::{Verdict, evaluate};
use crate::routes::RouteTable;
use crate::tenant::cache::TenantAuthCache;
use crate::tenant::repository::TenantRepository;
use axum::Json;
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

#[derive(Debug, Clone)]
pub struct ReplayArtifacts {
    pub snapshot_hash: String,
    pub request_body_json: serde_json::Value,
    pub request_headers: serde_json::Value,
    pub response_status: Option<u16>,
    pub response_headers: Option<serde_json::Value>,
    pub response_body: Option<String>,
    pub response_body_sha256: Option<String>,
    pub response_body_truncated: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub routes: Arc<RwLock<RouteTable>>,
    pub policies: Arc<RwLock<PolicySet>>,
    pub http_client: Client,
    pub audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
    pub route_config_hash: String,
    pub policy_set_hash: String,
    pub admin_token: String,
    pub tenant_repo: crate::tenant::repository::TenantRepository,
    pub tenant_cache: crate::tenant::cache::TenantAuthCache,
    pub rate_limiter: crate::auth::rate_limit::TenantRateLimiter,
    pub replay: ReplayConfig,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

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

#[allow(dead_code)]
async fn emit_and_persist(
    record: ExecutionRecord,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
) {
    ExecutionLog::from(&record).emit();
    if let Some(store) = audit_store {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(250),
            store.insert_execution(&record),
        )
        .await;
    }
}

fn spawn_emit_and_persist(
    record: ExecutionRecord,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
) {
    let log = ExecutionLog::from(&record);
    log.emit();
    if let Some(store) = audit_store {
        tokio::spawn(async move {
            if let Err(e) = store.insert_execution(&record).await {
                eprintln!("Failed to persist execution: {}", e);
            }
        });
    }
}

fn spawn_emit_and_persist_bundle(
    record: ExecutionRecord,
    artifacts: Option<ReplayArtifacts>,
    snapshot: Option<snapshot::PolicySnapshotRecord>,
    audit_store: Option<crate::storage::postgres::PostgresAuditStore>,
) {
    let log = ExecutionLog::from(&record);
    log.emit();
    if let Some(store) = audit_store {
        tokio::spawn(async move {
            if let Err(e) = store.insert_execution_bundle(&record, artifacts.as_ref(), snapshot.as_ref()).await {
                eprintln!("Failed to persist execution bundle: {}", e);
            }
        });
    }
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

    // 2. Tenant authentication
    let snapshot = state.tenant_cache.snapshot().await;
    let route_is_bound = snapshot.route_bindings.contains_key(&route_id);

    let (tenant_id, api_key_id, auth_outcome) = if !route_is_bound {
        (None, None, None)
    } else {
        let auth_result = authenticate_tenant_request(&headers, &state.tenant_cache).await;
        match auth_result {
            Ok(RequestAuthContext::Tenant {
                tenant_id,
                api_key_id,
                ..
            }) => {
                let bound_tenant_id = snapshot.route_bindings.get(&route_id);
                if bound_tenant_id.is_none() || bound_tenant_id != Some(&tenant_id) {
                    return (axum::http::StatusCode::NOT_FOUND, "Route not found").into_response();
                }
                if !state.rate_limiter.allow(tenant_id).await {
                    let record = ExecutionRecord {
                        execution_id: execution_id.clone(),
                        execution_started_at,
                        route_id: route_id.clone(),
                        tenant_id: Some(tenant_id),
                        api_key_id: Some(api_key_id),
                        auth_outcome: Some("rate_limited".to_string()),
                        upstream_url: Some(route.upstream.clone()),
                        method: method.to_string(),
                        source_ip: source_ip.clone(),
                        content_type,
                        user_agent,
                        had_authorization_header,
                        request_size_bytes,
                        request_body_sha256,
                        verdict: ExecutionVerdict::Rejected,
                        rejection_reason: Some("rate_limit_exceeded".to_string()),
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
                    return (
                        axum::http::StatusCode::TOO_MANY_REQUESTS,
                        "Rate limit exceeded",
                    )
                        .into_response();
                }
                (Some(tenant_id), Some(api_key_id), None)
            }
            Ok(RequestAuthContext::Admin) => (None, None, Some("admin".to_string())),
            Err(auth_failure) => {
                let auth_outcome_str = auth_failure.as_str().to_string();
                let record = ExecutionRecord {
                    execution_id: execution_id.clone(),
                    execution_started_at,
                    route_id: route_id.clone(),
                    tenant_id: None,
                    api_key_id: None,
                    auth_outcome: Some(auth_outcome_str.clone()),
                    upstream_url: Some(route.upstream.clone()),
                    method: method.to_string(),
                    source_ip: source_ip.clone(),
                    content_type,
                    user_agent,
                    had_authorization_header,
                    request_size_bytes,
                    request_body_sha256,
                    verdict: ExecutionVerdict::Rejected,
                    rejection_reason: Some(auth_outcome_str),
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
                return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            }
        }
    };

    // 3. Method check
    let method_str = method.to_string();
    if !route.methods.contains(&method_str) {
        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            execution_started_at,
            route_id: route_id.clone(),
            tenant_id,
            api_key_id,
            auth_outcome,
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
                tenant_id,
                api_key_id,
                auth_outcome,
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
                tenant_id,
                api_key_id,
                auth_outcome,
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

            if state.replay.enabled {
                let capture_request_headers =
                    snapshot::filter_headers(&headers, &state.replay.capture_request_headers);
                let policies = state.policies.read().await;
                let snapshot = snapshot::build_snapshot_from_set(&route, &*policies)
                    .unwrap_or_else(|_| snapshot::build_snapshot(&route, &[]));
                drop(policies);
                let artifacts = ReplayArtifacts {
                    snapshot_hash: snapshot.snapshot_hash.clone(),
                    request_body_json: payload.clone(),
                    request_headers: capture_request_headers,
                    response_status: None,
                    response_headers: None,
                    response_body: None,
                    response_body_sha256: None,
                    response_body_truncated: false,
                };
                spawn_emit_and_persist_bundle(
                    record,
                    Some(artifacts),
                    Some(snapshot),
                    state.audit_store.clone(),
                );
            } else {
                spawn_emit_and_persist(record, state.audit_store.clone());
            }
            return response::block_response(&execution_id, &policy_name, &rule_field, &message);
        }
        Verdict::Allow => {}
    }

    // 5. Forward to upstream
    let forward_start = Instant::now();

    let (snapshot, request_headers_for_artifact) = if state.replay.enabled {
        let capture_request_headers =
            snapshot::filter_headers(&headers, &state.replay.capture_request_headers);
        let policies = state.policies.read().await;
        let sp = snapshot::build_snapshot_from_set(&route, &*policies)
            .unwrap_or_else(|_| snapshot::build_snapshot(&route, &[]));
        drop(policies);
        (Some(sp), Some(capture_request_headers))
    } else {
        (None, None)
    };

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
                tenant_id,
                api_key_id,
                auth_outcome,
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

if state.replay.enabled {
                let response_body_bytes = result.body_bytes;
                let response_body_str = String::from_utf8_lossy(&response_body_bytes).to_string();
                let max_len = state.replay.max_response_body_bytes;
                let (response_body, response_body_sha256, truncated) = if response_body_str.len() > max_len {
                    let truncated_body = response_body_str[..max_len].to_string();
                    let sha = Some(hash_string(&truncated_body));
                    (truncated_body, sha, true)
                } else {
                    let sha = Some(hash_string(&response_body_str));
                    (response_body_str.clone(), sha, false)
                };

                let response_headers = snapshot::filter_headers(
                    result.response.headers(),
                    &state.replay.capture_response_headers,
                );

                let artifacts = ReplayArtifacts {
                    snapshot_hash: snapshot.as_ref().unwrap().snapshot_hash.clone(),
                    request_body_json: payload.clone(),
                    request_headers: request_headers_for_artifact.unwrap(),
                    response_status: Some(result.status),
                    response_headers: Some(response_headers),
                    response_body: Some(response_body),
                    response_body_sha256,
                    response_body_truncated: truncated,
                };
                spawn_emit_and_persist_bundle(
                    record,
                    Some(artifacts),
                    snapshot,
                    state.audit_store.clone(),
                );
            } else {
                spawn_emit_and_persist(record, state.audit_store.clone());
            }
            result.response
        }
        Err(e) => {
            let record = ExecutionRecord {
                execution_id: execution_id.clone(),
                execution_started_at,
                route_id: route_id.clone(),
                tenant_id,
                api_key_id,
                auth_outcome,
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::require_audit_access,
        ));

    let audit_admin_routes = axum::Router::new()
        .route(
            "/v1/audit/integrity",
            axum::routing::get(crate::audit::api::verify_integrity),
        )
        .layer(axum::middleware::from_fn_with_state(
            admin_token.clone(),
            crate::auth::middleware::require_admin_token,
        ));

    let admin_routes = axum::Router::new()
        .route(
            "/v1/admin/tenants",
            axum::routing::post(
                |State(state): State<AppState>,
                 Json(request): Json<super::tenant::api::CreateTenantRequest>| async move {
                    super::tenant::api::create_tenant(State(state), Json(request)).await
                },
            )
            .get(|State(state): State<AppState>| async move {
                super::tenant::api::list_tenants(State(state)).await
            }),
        )
        .route(
            "/v1/admin/tenants/{tenant_id}/keys",
            axum::routing::post(
                |State(state): State<AppState>,
                 Path(tenant_id): Path<String>,
                 Json(request): Json<super::tenant::api::CreateApiKeyRequest>| async move {
                    super::tenant::api::create_api_key(State(state), Path(tenant_id), Json(request))
                        .await
                },
            )
            .get(
                |State(state): State<AppState>, Path(tenant_id): Path<String>| async move {
                    super::tenant::api::list_api_keys(State(state), Path(tenant_id)).await
                },
            ),
        )
        .route(
            "/v1/admin/tenants/{tenant_id}/keys/{key_id}/revoke",
            axum::routing::post(
                |State(state): State<AppState>,
                 Path((tenant_id, key_id)): Path<(String, String)>,
                 Json(request): Json<super::tenant::api::RevokeApiKeyRequest>| async move {
                    super::tenant::api::revoke_api_key(
                        State(state),
                        Path((tenant_id, key_id)),
                        Json(request),
                    )
                    .await
                },
            ),
        )
        .route(
            "/v1/admin/tenants/{tenant_id}/routes",
            axum::routing::post(
                |State(state): State<AppState>,
                 Path(tenant_id): Path<String>,
                 Json(request): Json<super::tenant::api::BindRouteRequest>| async move {
                    super::tenant::api::bind_route(State(state), Path(tenant_id), Json(request))
                        .await
                },
            ),
        )
        .layer(axum::middleware::from_fn_with_state(
            admin_token.clone(),
            crate::auth::middleware::require_admin_token,
        ));

    let main_router = axum::Router::new()
        .route("/v1/execute/{route_id}", axum::routing::any(handle_execute))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(audit_routes)
        .merge(audit_admin_routes)
        .merge(admin_routes)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            request_body_limit_bytes,
        ));

    main_router.with_state(state)
}

impl AppState {
    #[allow(dead_code)]
    pub fn for_admin_router() -> Self {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost:5432")
            .unwrap();
        Self {
            routes: Arc::new(RwLock::new(RouteTable::from_routes(vec![]))),
            policies: Arc::new(RwLock::new(PolicySet::new())),
            http_client: Client::new(),
            audit_store: None,
            route_config_hash: String::new(),
            policy_set_hash: String::new(),
            admin_token: String::new(),
            tenant_repo: TenantRepository::new(pool),
            tenant_cache: TenantAuthCache::default(),
            rate_limiter: TenantRateLimiter::new(0, 0),
            replay: ReplayConfig::default(),
        }
    }
}

pub async fn refresh_tenant_auth_cache(state: &AppState) -> Result<(), axum::http::StatusCode> {
    let snapshot = state
        .tenant_repo
        .load_auth_snapshot()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let routes = state.routes.read().await;
    crate::tenant::cache::validate_all_routes_bound(&routes, &snapshot)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    drop(routes);
    state.tenant_cache.replace(snapshot).await;
    Ok(())
}

#[allow(dead_code)]
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

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost:5432/test");

    AppState {
        routes: Arc::new(RwLock::new(route_table)),
        policies: Arc::new(RwLock::new(policy_set)),
        http_client,
        audit_store,
        route_config_hash,
        policy_set_hash,
        admin_token: String::new(),
        tenant_repo: pool
            .ok()
            .map(crate::tenant::repository::TenantRepository::new)
            .unwrap_or_else(|| {
                crate::tenant::repository::TenantRepository::new(
                    sqlx::postgres::PgPoolOptions::new()
                        .max_connections(1)
                        .connect_lazy("")
                        .unwrap(),
                )
            }),
        tenant_cache: crate::tenant::cache::TenantAuthCache::default(),
        rate_limiter: crate::auth::rate_limit::TenantRateLimiter::new(120, 30),
        replay: ReplayConfig::default(),
    }
}

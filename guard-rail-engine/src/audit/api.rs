use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::storage::postgres::ExecutionAuditRow;

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    pub route_id: Option<String>,
    pub verdict: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub cursor: Option<i64>,
    pub order: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AuditListResponse {
    pub items: Vec<ExecutionAuditRow>,
    pub total: i64,
    pub next_cursor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct IntegrityQuery {
    pub from_execution_id: String,
    pub to_execution_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct IntegrityResponse {
    pub chain_valid: bool,
    pub first_invalid_record: Option<String>,
    pub checked_from: String,
    pub checked_to: String,
}

pub async fn list_executions(
    State(state): State<crate::proxy::AppState>,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, StatusCode> {
    let store = state
        .audit_store
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let page = store
        .list_executions(query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(page))
}

pub async fn get_execution(
    State(state): State<crate::proxy::AppState>,
    Path(execution_id): Path<String>,
) -> Result<Json<ExecutionAuditRow>, StatusCode> {
    let store = state
        .audit_store
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let row = store
        .get_execution_by_id(&execution_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

pub async fn verify_integrity(
    State(state): State<crate::proxy::AppState>,
    Query(query): Query<IntegrityQuery>,
) -> Result<Json<IntegrityResponse>, StatusCode> {
    let store = state
        .audit_store
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let result = store
        .verify_integrity(query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}

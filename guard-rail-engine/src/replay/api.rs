use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};

use crate::auth::context::AuditAccess;
use crate::replay::engine::{ReplayRequest, ReplayResult, map_replay_error};

pub async fn create_replay(
    State(state): State<crate::proxy::AppState>,
    Extension(access): Extension<AuditAccess>,
    Path(execution_id): Path<String>,
    Json(request): Json<ReplayRequest>,
) -> Result<Json<ReplayResult>, StatusCode> {
    authorize_replay(&state, &access, &execution_id).await?;

    let result = crate::replay::engine::replay_execution(&state, &execution_id, request.policy_source)
        .await
        .map_err(map_replay_error)?;
    Ok(Json(result))
}

pub async fn get_replay_summary(
    State(state): State<crate::proxy::AppState>,
    Extension(access): Extension<AuditAccess>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    authorize_replay(&state, &access, &execution_id).await?;

    let store = state
        .audit_store
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let artifacts = store
        .get_execution_artifacts(&execution_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "execution_id": execution_id,
        "snapshot_hash": artifacts.snapshot_hash,
        "has_response_artifacts": artifacts.response_status.is_some(),
    })))
}

async fn authorize_replay(
    state: &crate::proxy::AppState,
    access: &AuditAccess,
    execution_id: &str,
) -> Result<(), StatusCode> {
    let store = state
        .audit_store
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    match access {
        AuditAccess::Admin => Ok(()),
        AuditAccess::Tenant { tenant_id } => {
            let execution = store
                .get_execution_by_id(execution_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?;

            if execution.tenant_id == Some(*tenant_id) {
                Ok(())
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}
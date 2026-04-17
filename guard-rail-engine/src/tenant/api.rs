use axum::{
    extract::{Path, State},
    response::Json,
};

#[derive(Debug, serde::Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BindRouteRequest {
    pub route_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct RevokeApiKeyRequest {
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct TenantResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub disabled_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<crate::tenant::Tenant> for TenantResponse {
    fn from(t: crate::tenant::Tenant) -> Self {
        Self {
            id: t.id,
            name: t.name,
            status: t.status,
            created_at: t.created_at,
            disabled_at: t.disabled_at,
        }
    }
}

pub async fn create_tenant(
    State(state): State<crate::proxy::AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, axum::http::StatusCode> {
    let tenant = state
        .tenant_repo
        .create_tenant(&request.name)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::proxy::refresh_tenant_auth_cache(&state).await?;
    Ok(Json(TenantResponse::from(tenant)))
}

pub async fn list_tenants(
    State(state): State<crate::proxy::AppState>,
) -> Result<Json<Vec<TenantResponse>>, axum::http::StatusCode> {
    let tenants = state
        .tenant_repo
        .list_tenants()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tenants.into_iter().map(TenantResponse::from).collect()))
}

#[derive(Debug, serde::Serialize)]
pub struct ApiKeyResponse {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub raw_key: String,
}

pub async fn create_api_key(
    State(state): State<crate::proxy::AppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, axum::http::StatusCode> {
    let tenant_id = tenant_id.parse::<uuid::Uuid>().map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let key = state
        .tenant_repo
        .create_api_key(tenant_id, &request.name)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::proxy::refresh_tenant_auth_cache(&state).await?;
    Ok(Json(ApiKeyResponse {
        id: key.id,
        tenant_id: key.tenant_id,
        name: key.name,
        key_prefix: key.key_prefix,
        raw_key: key.raw_key,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct ApiKeyListItem {
    pub id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn list_api_keys(
    State(state): State<crate::proxy::AppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Vec<ApiKeyListItem>>, axum::http::StatusCode> {
    let tenant_id = tenant_id.parse::<uuid::Uuid>().map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let keys = state
        .tenant_repo
        .list_api_keys(tenant_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(keys))
}

pub async fn revoke_api_key(
    State(state): State<crate::proxy::AppState>,
    Path((_tenant_id, key_id)): Path<(String, String)>,
    Json(request): Json<RevokeApiKeyRequest>,
) -> Result<Json<()>, axum::http::StatusCode> {
    let key_id = key_id.parse::<uuid::Uuid>().map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    state
        .tenant_repo
        .revoke_api_key(key_id, request.reason.as_deref())
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::proxy::refresh_tenant_auth_cache(&state).await?;
    Ok(Json(()))
}

pub async fn bind_route(
    State(state): State<crate::proxy::AppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<BindRouteRequest>,
) -> Result<Json<()>, axum::http::StatusCode> {
    let tenant_id = tenant_id.parse::<uuid::Uuid>().map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    state
        .tenant_repo
        .bind_route(&request.route_id, tenant_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::proxy::refresh_tenant_auth_cache(&state).await?;
    Ok(Json(()))
}
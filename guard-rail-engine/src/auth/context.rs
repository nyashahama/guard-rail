use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum RequestAuthContext {
    Admin,
    Tenant {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
        key_prefix: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAccess {
    Admin,
    Tenant { tenant_id: uuid::Uuid },
}

#[derive(Debug, Clone)]
pub enum RequestAuthFailure {
    MissingApiKey,
    InvalidApiKey,
    RevokedApiKey {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
    },
    TenantDisabled {
        tenant_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
    },
}

impl RequestAuthFailure {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestAuthFailure::MissingApiKey => "missing_api_key",
            RequestAuthFailure::InvalidApiKey => "invalid_api_key",
            RequestAuthFailure::RevokedApiKey { .. } => "revoked_api_key",
            RequestAuthFailure::TenantDisabled { .. } => "tenant_disabled",
        }
    }
}

pub async fn authenticate_tenant_request(
    headers: &axum::http::HeaderMap,
    cache: &crate::tenant::cache::TenantAuthCache,
) -> Result<RequestAuthContext, RequestAuthFailure> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let raw_key = header
        .strip_prefix("Bearer ")
        .ok_or(RequestAuthFailure::MissingApiKey)?;

    if raw_key.is_empty() {
        return Err(RequestAuthFailure::MissingApiKey);
    }

    let hashed = crate::auth::api_keys::hash_api_key(raw_key);
    let snapshot = cache.snapshot().await;
    let cached = snapshot
        .api_keys
        .get(&hashed)
        .ok_or(RequestAuthFailure::InvalidApiKey)?;

    if cached.tenant_status != "active" {
        return Err(RequestAuthFailure::TenantDisabled {
            tenant_id: cached.tenant_id,
            api_key_id: cached.id,
        });
    }

    Ok(RequestAuthContext::Tenant {
        tenant_id: cached.tenant_id,
        api_key_id: cached.id,
        key_prefix: cached.key_prefix.clone(),
    })
}

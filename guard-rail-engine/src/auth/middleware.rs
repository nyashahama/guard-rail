use axum::{extract::State, http::StatusCode, middleware::Next, response::Response};

pub async fn require_admin_token(
    State(expected_token): State<String>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", expected_token);
    if header != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

pub async fn require_audit_access(
    State(state): State<crate::proxy::AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let tenant =
        crate::auth::context::authenticate_tenant_request(request.headers(), &state.tenant_cache)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut request = request;
    if let crate::auth::context::RequestAuthContext::Tenant { tenant_id, .. } = tenant {
        request
            .extensions_mut()
            .insert(crate::auth::context::AuditAccess::Tenant { tenant_id });
    }
    Ok(next.run(request).await)
}

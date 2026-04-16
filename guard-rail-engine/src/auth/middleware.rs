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

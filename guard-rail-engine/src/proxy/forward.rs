use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use reqwest::Client;
use std::time::Duration;

pub struct ForwardResult {
    pub status: u16,
    pub response: Response,
}

pub async fn forward_request(
    client: &Client,
    upstream_url: &str,
    method: &str,
    body: bytes::Bytes,
    original_headers: &HeaderMap,
    execution_id: &str,
    timeout_ms: u64,
) -> Result<ForwardResult, String> {
    let mut headers = original_headers.clone();

    // Strip Authorization header — don't leak Guard Rail API key to upstream
    headers.remove("authorization");

    // Add Guard Rail headers
    if let Ok(val) = HeaderValue::from_str(execution_id) {
        headers.insert(HeaderName::from_static("x-guardrail-execution-id"), val);
    }
    headers.insert(
        HeaderName::from_static("x-guardrail-verdict"),
        HeaderValue::from_static("ALLOW"),
    );

    // Remove host header — reqwest will set it from the URL
    headers.remove("host");

    let reqwest_method = method
        .parse::<reqwest::Method>()
        .map_err(|e| format!("Invalid method: {}", e))?;

    let response = client
        .request(reqwest_method, upstream_url)
        .headers(reqwest_headers(&headers))
        .body(body)
        .timeout(Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|e| format!("Upstream request failed: {}", e))?;

    let status = response.status().as_u16();
    let resp_headers = response.headers().clone();
    let resp_body = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read upstream response: {}", e))?;

    // Build axum response from upstream response
    let mut builder = axum::http::Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));

    // Copy upstream response headers
    for (key, value) in resp_headers.iter() {
        builder = builder.header(key.as_str(), value.as_bytes());
    }

    // Add execution ID header to response
    builder = builder.header("x-guardrail-execution-id", execution_id);

    let response = builder
        .body(axum::body::Body::from(resp_body))
        .map_err(|e| format!("Failed to build response: {}", e))?
        .into_response();

    Ok(ForwardResult { status, response })
}

fn reqwest_headers(axum_headers: &HeaderMap) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in axum_headers.iter() {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_ref())
            && let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
        {
            headers.insert(name, val);
        }
    }
    headers
}

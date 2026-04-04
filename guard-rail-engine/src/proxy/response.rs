use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub status: String,
    pub execution_id: String,
    pub policy: String,
    pub rule_field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RejectResponse {
    pub status: String,
    pub execution_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: String,
    pub execution_id: String,
    pub message: String,
}

pub fn block_response(
    execution_id: &str,
    policy: &str,
    rule_field: &str,
    message: &str,
) -> Response {
    let body = BlockResponse {
        status: "blocked".to_string(),
        execution_id: execution_id.to_string(),
        policy: policy.to_string(),
        rule_field: rule_field.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::FORBIDDEN,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn reject_response(execution_id: &str, message: &str) -> Response {
    let body = RejectResponse {
        status: "rejected".to_string(),
        execution_id: execution_id.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::BAD_REQUEST,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn bad_gateway_response(execution_id: &str, message: &str) -> Response {
    let body = ErrorResponse {
        status: "error".to_string(),
        execution_id: execution_id.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::BAD_GATEWAY,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn method_not_allowed_response(execution_id: &str) -> Response {
    let body = RejectResponse {
        status: "rejected".to_string(),
        execution_id: execution_id.to_string(),
        message: "Method not allowed for this route".to_string(),
    };
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

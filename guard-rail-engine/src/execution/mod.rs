use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionVerdict {
    Rejected,
    Blocked,
    Allowed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub execution_started_at: chrono::DateTime<Utc>,
    pub route_id: String,
    pub tenant_id: Option<uuid::Uuid>,
    pub api_key_id: Option<uuid::Uuid>,
    pub auth_outcome: Option<String>,
    pub upstream_url: Option<String>,
    pub method: String,
    pub source_ip: String,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub had_authorization_header: bool,
    pub request_size_bytes: usize,
    pub request_body_sha256: String,
    pub verdict: ExecutionVerdict,
    pub rejection_reason: Option<String>,
    pub matched_policy_name: Option<String>,
    pub matched_rule_field: Option<String>,
    pub matched_rule_condition: Option<String>,
    pub matched_rule_severity: Option<String>,
    pub violation_value_hash: Option<String>,
    pub violation_value_preview: Option<String>,
    pub upstream_status: Option<u16>,
    pub forward_error: Option<String>,
    pub latency_inspect_us: u128,
    pub latency_forward_ms: Option<u128>,
    pub latency_total_ms: u128,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

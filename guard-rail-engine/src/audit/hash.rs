use crate::execution::ExecutionRecord;

pub fn hash_body(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw))
}

pub fn hash_string(value: &str) -> String {
    hash_body(value.as_bytes())
}

pub fn preview_violation_value(value: &str) -> Option<String> {
    if let Ok(url) = url::Url::parse(value)
        && let Some(host) = url.host_str()
    {
        return Some(format!("{}://{}", url.scheme(), host));
    }

    if let Some((local, domain)) = value.split_once('@')
        && !local.is_empty() && !domain.is_empty()
    {
        return Some(format!("{}***@{}", &local[..1], domain));
    }

    if value.len() >= 8 {
        return Some(format!("{}***{}", &value[..4], &value[value.len() - 4..]));
    }

    None
}

pub fn record_hash(record: &ExecutionRecord, previous_hash: Option<&str>) -> String {
    let canonical = serde_json::json!({
        "execution_id": record.execution_id,
        "execution_started_at": record.execution_started_at,
        "route_id": record.route_id,
        "upstream_url": record.upstream_url,
        "method": record.method,
        "source_ip": record.source_ip,
        "content_type": record.content_type,
        "user_agent": record.user_agent,
        "had_authorization_header": record.had_authorization_header,
        "request_size_bytes": record.request_size_bytes,
        "request_body_sha256": record.request_body_sha256,
        "verdict": record.verdict,
        "rejection_reason": record.rejection_reason,
        "matched_policy_name": record.matched_policy_name,
        "matched_rule_field": record.matched_rule_field,
        "matched_rule_condition": record.matched_rule_condition,
        "matched_rule_severity": record.matched_rule_severity,
        "violation_value_hash": record.violation_value_hash,
        "violation_value_preview": record.violation_value_preview,
        "upstream_status": record.upstream_status,
        "forward_error": record.forward_error,
        "latency_inspect_us": record.latency_inspect_us,
        "latency_forward_ms": record.latency_forward_ms,
        "latency_total_ms": record.latency_total_ms,
        "route_config_hash": record.route_config_hash,
        "policy_set_hash": record.policy_set_hash,
        "previous_hash": previous_hash,
    });

    hash_body(canonical.to_string().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_body_uses_raw_bytes() {
        let hash_a = hash_body(br#"{"a":1}"#);
        let hash_b = hash_body(br#"{ "a": 1 }"#);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn test_preview_url_redacts_path_and_query() {
        let preview = preview_violation_value("https://evil.sh/exfil?token=abc");
        assert_eq!(preview.as_deref(), Some("https://evil.sh"));
    }

    #[test]
    fn test_record_hash_changes_when_previous_hash_changes() {
        let record = ExecutionRecord {
            execution_id: "GR-EXE-1".to_string(),
            route_id: "transfer-api".to_string(),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            upstream_url: Some("https://internal/api".to_string()),
            content_type: Some("application/json".to_string()),
            user_agent: None,
            had_authorization_header: false,
            request_size_bytes: 14,
            request_body_sha256: hash_body(br#"{"amount":10}"#),
            execution_started_at: chrono::Utc::now(),
            verdict: ExecutionVerdict::Blocked,
            rejection_reason: None,
            matched_policy_name: Some("block-callbacks".to_string()),
            matched_rule_field: Some("$.callback".to_string()),
            matched_rule_condition: Some("domain_not_in".to_string()),
            matched_rule_severity: Some("critical".to_string()),
            violation_value_hash: Some(hash_string("https://evil.sh/exfil")),
            violation_value_preview: Some("https://evil.sh".to_string()),
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 10,
            latency_forward_ms: None,
            latency_total_ms: 1,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };

        let first = record_hash(&record, None);
        let second = record_hash(&record, Some("previous-hash"));
        assert_ne!(first, second);
    }
}

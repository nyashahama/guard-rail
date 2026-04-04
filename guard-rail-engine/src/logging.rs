use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ExecutionLog {
    pub execution_id: String,
    pub timestamp: String,
    pub route_id: String,
    pub method: String,
    pub source_ip: String,
    pub verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_error: Option<String>,
    pub latency_inspect_us: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_forward_ms: Option<u128>,
    pub latency_total_ms: u128,
}

impl ExecutionLog {
    pub fn new(execution_id: String, route_id: String, method: String, source_ip: String) -> Self {
        ExecutionLog {
            execution_id,
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            route_id,
            method,
            source_ip,
            verdict: String::new(),
            policy: None,
            rule_field: None,
            violation_value: None,
            upstream: None,
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 0,
            latency_forward_ms: None,
            latency_total_ms: 0,
        }
    }

    pub fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{}", json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_log_serializes_to_json() {
        let mut log = ExecutionLog::new(
            "GR-EXE-abc123".to_string(),
            "transfer-api".to_string(),
            "POST".to_string(),
            "127.0.0.1".to_string(),
        );
        log.verdict = "BLOCKED".to_string();
        log.policy = Some("block-callbacks".to_string());
        log.rule_field = Some("$.callback".to_string());
        log.latency_inspect_us = 142;
        log.latency_total_ms = 1;

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"execution_id\":\"GR-EXE-abc123\""));
        assert!(json.contains("\"verdict\":\"BLOCKED\""));
        assert!(json.contains("\"policy\":\"block-callbacks\""));
        assert!(!json.contains("\"upstream\""));
    }

    #[test]
    fn test_execution_log_allowed_with_forward() {
        let mut log = ExecutionLog::new(
            "GR-EXE-def456".to_string(),
            "partner".to_string(),
            "POST".to_string(),
            "10.0.0.1".to_string(),
        );
        log.verdict = "ALLOWED".to_string();
        log.upstream = Some("https://erp.internal/webhook".to_string());
        log.upstream_status = Some(200);
        log.latency_inspect_us = 80;
        log.latency_forward_ms = Some(45);
        log.latency_total_ms = 46;

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"verdict\":\"ALLOWED\""));
        assert!(json.contains("\"upstream_status\":200"));
        assert!(json.contains("\"latency_forward_ms\":45"));
    }
}

use crate::execution::ExecutionRecord;
use crate::execution::ExecutionVerdict;
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

impl From<&ExecutionRecord> for ExecutionLog {
    fn from(record: &ExecutionRecord) -> Self {
        ExecutionLog {
            execution_id: record.execution_id.clone(),
            timestamp: record
                .execution_started_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            route_id: record.route_id.clone(),
            method: record.method.clone(),
            source_ip: record.source_ip.clone(),
            verdict: match record.verdict {
                ExecutionVerdict::Rejected => "REJECTED".to_string(),
                ExecutionVerdict::Blocked => "BLOCKED".to_string(),
                ExecutionVerdict::Allowed => "ALLOWED".to_string(),
            },
            policy: record.matched_policy_name.clone(),
            rule_field: record.matched_rule_field.clone(),
            violation_value: record.violation_value_preview.clone(),
            upstream: record.upstream_url.clone(),
            upstream_status: record.upstream_status,
            forward_error: record.forward_error.clone(),
            latency_inspect_us: record.latency_inspect_us,
            latency_forward_ms: record.latency_forward_ms,
            latency_total_ms: record.latency_total_ms,
        }
    }
}

impl ExecutionLog {
    #[allow(dead_code)]
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
            tracing::info!(
                execution_id = %self.execution_id,
                route_id = %self.route_id,
                method = %self.method,
                source_ip = %self.source_ip,
                verdict = %self.verdict,
                payload = %json,
                "execution log"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::{Layer, Registry};

    #[derive(Default)]
    struct FieldCollector {
        fields: BTreeMap<String, String>,
    }

    impl Visit for FieldCollector {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    #[derive(Clone, Default)]
    struct CaptureLayer {
        events: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut collector = FieldCollector::default();
            event.record(&mut collector);
            self.events
                .lock()
                .expect("capture layer poisoned")
                .push(collector.fields);
        }
    }

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

    #[test]
    fn test_execution_log_from_record() {
        use crate::audit::hash::{hash_body, hash_string};

        let record = ExecutionRecord {
            execution_id: "GR-EXE-123".to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "transfer-api".to_string(),
            tenant_id: None,
            api_key_id: None,
            auth_outcome: None,
            upstream_url: Some("https://internal.api".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: None,
            had_authorization_header: true,
            request_size_bytes: 100,
            request_body_sha256: hash_body(b"test body"),
            verdict: ExecutionVerdict::Blocked,
            rejection_reason: Some("policy violation".to_string()),
            matched_policy_name: Some("block-transfer".to_string()),
            matched_rule_field: Some("$.amount".to_string()),
            matched_rule_condition: Some("gt".to_string()),
            matched_rule_severity: Some("high".to_string()),
            violation_value_hash: Some(hash_string("https://evil.com")),
            violation_value_preview: Some("https://evil.com".to_string()),
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 50,
            latency_forward_ms: None,
            latency_total_ms: 2,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };

        let log = ExecutionLog::from(&record);
        assert_eq!(log.execution_id, "GR-EXE-123");
        assert_eq!(log.verdict, "BLOCKED");
        assert_eq!(log.policy, Some("block-transfer".to_string()));
        assert_eq!(log.violation_value, Some("https://evil.com".to_string()));
    }

    #[test]
    fn test_execution_log_emit_uses_tracing_fields() {
        use crate::execution::{ExecutionRecord, ExecutionVerdict};
        use uuid::Uuid;

        let record = ExecutionRecord {
            execution_id: "GR-EXE-555".to_string(),
            execution_started_at: chrono::Utc::now(),
            route_id: "route-a".to_string(),
            tenant_id: Some(Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()),
            api_key_id: Some(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
            auth_outcome: None,
            upstream_url: Some("https://internal".to_string()),
            method: "POST".to_string(),
            source_ip: "127.0.0.1".to_string(),
            content_type: Some("application/json".to_string()),
            user_agent: Some("test".to_string()),
            had_authorization_header: true,
            request_size_bytes: 20,
            request_body_sha256: "hash".to_string(),
            verdict: ExecutionVerdict::Allowed,
            rejection_reason: None,
            matched_policy_name: None,
            matched_rule_field: None,
            matched_rule_condition: None,
            matched_rule_severity: None,
            violation_value_hash: None,
            violation_value_preview: None,
            upstream_status: Some(200),
            forward_error: None,
            latency_inspect_us: 10,
            latency_forward_ms: Some(5),
            latency_total_ms: 5,
            route_config_hash: "route-hash".to_string(),
            policy_set_hash: "policy-hash".to_string(),
        };

        let capture = CaptureLayer::default();
        let events = capture.events.clone();
        let subscriber = Registry::default().with(capture);

        tracing::subscriber::with_default(subscriber, || {
            let log = ExecutionLog::from(&record);
            log.emit();
        });

        let events = events.lock().expect("capture layer poisoned");
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.get("execution_id"), Some(&"GR-EXE-555".to_string()));
        assert_eq!(event.get("route_id"), Some(&"route-a".to_string()));
        assert_eq!(event.get("method"), Some(&"POST".to_string()));
        assert_eq!(event.get("source_ip"), Some(&"127.0.0.1".to_string()));
        assert_eq!(event.get("verdict"), Some(&"ALLOWED".to_string()));
        assert!(
            event
                .get("payload")
                .expect("payload field present")
                .contains("\"execution_id\":\"GR-EXE-555\"")
        );
        assert!(
            event
                .get("payload")
                .expect("payload field present")
                .contains("\"route_id\":\"route-a\"")
        );
        assert_eq!(event.get("message"), Some(&"execution log".to_string()));
    }
}

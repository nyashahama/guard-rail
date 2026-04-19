use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

pub struct Metrics {
    registry: Registry,
    requests_total: IntCounterVec,
    request_latency_seconds: HistogramVec,
    upstream_failures_total: IntCounterVec,
    audit_persist_failures_total: IntCounterVec,
    replay_persist_failures_total: IntCounterVec,
    inflight_requests: IntGauge,
    readiness: IntGauge,
    shutdown_transitions_total: IntCounterVec,
}

pub struct InflightGuard {
    inflight_requests: IntGauge,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.inflight_requests.dec();
    }
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let requests_total = IntCounterVec::new(
            Opts::new(
                "guardrail_requests_total",
                "Total handled requests grouped by route, method, and verdict.",
            ),
            &["route_id", "method", "verdict"],
        )?;
        let request_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "guardrail_request_latency_seconds",
                "Observed request execution latency in seconds.",
            ),
            &["route_id", "method", "verdict"],
        )?;
        let upstream_failures_total = IntCounterVec::new(
            Opts::new(
                "guardrail_upstream_failures_total",
                "Total upstream forwarding failures.",
            ),
            &["route_id"],
        )?;
        let audit_persist_failures_total = IntCounterVec::new(
            Opts::new(
                "guardrail_audit_persist_failures_total",
                "Total audit persistence failures.",
            ),
            &["operation"],
        )?;
        let replay_persist_failures_total = IntCounterVec::new(
            Opts::new(
                "guardrail_replay_persist_failures_total",
                "Total replay persistence failures.",
            ),
            &["operation"],
        )?;
        let inflight_requests = IntGauge::new(
            "guardrail_inflight_requests",
            "Current in-flight request count.",
        )?;
        let readiness = IntGauge::new("guardrail_readiness", "Readiness state for the engine.")?;
        let shutdown_transitions_total = IntCounterVec::new(
            Opts::new(
                "guardrail_shutdown_transitions_total",
                "Total shutdown state transitions.",
            ),
            &["state"],
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(request_latency_seconds.clone()))?;
        registry.register(Box::new(upstream_failures_total.clone()))?;
        registry.register(Box::new(audit_persist_failures_total.clone()))?;
        registry.register(Box::new(replay_persist_failures_total.clone()))?;
        registry.register(Box::new(inflight_requests.clone()))?;
        registry.register(Box::new(readiness.clone()))?;
        registry.register(Box::new(shutdown_transitions_total.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            request_latency_seconds,
            upstream_failures_total,
            audit_persist_failures_total,
            replay_persist_failures_total,
            inflight_requests,
            readiness,
            shutdown_transitions_total,
        })
    }

    pub fn record_execution(
        &self,
        route_id: &str,
        method: &str,
        verdict: &str,
        latency_seconds: f64,
    ) {
        self.requests_total
            .with_label_values(&[route_id, method, verdict])
            .inc();
        self.request_latency_seconds
            .with_label_values(&[route_id, method, verdict])
            .observe(latency_seconds);
    }

    pub fn set_readiness(&self, ready: bool) {
        self.readiness.set(if ready { 1 } else { 0 });
    }

    pub fn inflight_guard(&self) -> InflightGuard {
        self.inflight_requests.inc();
        InflightGuard {
            inflight_requests: self.inflight_requests.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn inflight_requests(&self) -> i64 {
        self.inflight_requests.get()
    }

    pub fn render(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    pub fn record_upstream_failure(&self, route_id: &str) {
        self.upstream_failures_total
            .with_label_values(&[route_id])
            .inc();
    }

    pub fn record_audit_persist_failure(&self, operation: &str) {
        self.audit_persist_failures_total
            .with_label_values(&[operation])
            .inc();
    }

    pub fn record_replay_persist_failure(&self, operation: &str) {
        self.replay_persist_failures_total
            .with_label_values(&[operation])
            .inc();
    }

    pub fn record_shutdown_transition(&self, state: &str) {
        self.shutdown_transitions_total
            .with_label_values(&[state])
            .inc();
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn test_metrics_snapshot_contains_request_and_readiness_series() {
        let metrics = Metrics::new().unwrap();
        metrics.set_readiness(true);
        metrics.record_execution("test-route", "POST", "ALLOWED", 0.012);

        let snapshot = metrics.render().unwrap();
        assert!(snapshot.contains("guardrail_requests_total"));
        assert!(snapshot.contains("route_id=\"test-route\""));
        assert!(snapshot.contains("guardrail_readiness 1"));
    }

    #[test]
    fn test_metrics_snapshot_contains_failure_and_shutdown_series_after_updates() {
        let metrics = Metrics::new().unwrap();
        metrics.record_upstream_failure("route-a");
        metrics.record_audit_persist_failure("insert_execution");
        metrics.record_replay_persist_failure("insert_execution_bundle");
        metrics.record_shutdown_transition("ready");

        let snapshot = metrics.render().unwrap();
        assert!(snapshot.contains("guardrail_upstream_failures_total"));
        assert!(snapshot.contains("route_id=\"route-a\""));
        assert!(snapshot.contains("operation=\"insert_execution\""));
        assert!(snapshot.contains("operation=\"insert_execution_bundle\""));
        assert!(snapshot.contains("state=\"ready\""));
    }
}

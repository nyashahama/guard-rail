use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};

pub struct Metrics {
    registry: Registry,
    requests_total: IntCounterVec,
    request_latency_seconds: HistogramVec,
    upstream_failures_total: IntCounter,
    audit_persist_failures_total: IntCounter,
    replay_persist_failures_total: IntCounter,
    inflight_requests: IntGauge,
    readiness: IntGauge,
    shutdown_transitions_total: IntCounter,
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
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounterVec::new(
            Opts::new(
                "guardrail_requests_total",
                "Total handled requests grouped by route, method, and verdict.",
            ),
            &["route_id", "method", "verdict"],
        )
        .expect("create requests_total counter");
        let request_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "guardrail_request_latency_seconds",
                "Observed request execution latency in seconds.",
            ),
            &["route_id", "method", "verdict"],
        )
        .expect("create request_latency_seconds histogram");
        let upstream_failures_total = IntCounter::new(
            "guardrail_upstream_failures_total",
            "Total upstream forwarding failures.",
        )
        .expect("create upstream_failures_total counter");
        let audit_persist_failures_total = IntCounter::new(
            "guardrail_audit_persist_failures_total",
            "Total audit persistence failures.",
        )
        .expect("create audit_persist_failures_total counter");
        let replay_persist_failures_total = IntCounter::new(
            "guardrail_replay_persist_failures_total",
            "Total replay persistence failures.",
        )
        .expect("create replay_persist_failures_total counter");
        let inflight_requests = IntGauge::new(
            "guardrail_inflight_requests",
            "Current in-flight request count.",
        )
        .expect("create inflight_requests gauge");
        let readiness = IntGauge::new("guardrail_readiness", "Readiness state for the engine.")
            .expect("create readiness gauge");
        let shutdown_transitions_total = IntCounter::new(
            "guardrail_shutdown_transitions_total",
            "Total shutdown state transitions.",
        )
        .expect("create shutdown_transitions_total counter");

        registry
            .register(Box::new(requests_total.clone()))
            .expect("register requests_total");
        registry
            .register(Box::new(request_latency_seconds.clone()))
            .expect("register request_latency_seconds");
        registry
            .register(Box::new(upstream_failures_total.clone()))
            .expect("register upstream_failures_total");
        registry
            .register(Box::new(audit_persist_failures_total.clone()))
            .expect("register audit_persist_failures_total");
        registry
            .register(Box::new(replay_persist_failures_total.clone()))
            .expect("register replay_persist_failures_total");
        registry
            .register(Box::new(inflight_requests.clone()))
            .expect("register inflight_requests");
        registry
            .register(Box::new(readiness.clone()))
            .expect("register readiness");
        registry
            .register(Box::new(shutdown_transitions_total.clone()))
            .expect("register shutdown_transitions_total");

        Self {
            registry,
            requests_total,
            request_latency_seconds,
            upstream_failures_total,
            audit_persist_failures_total,
            replay_persist_failures_total,
            inflight_requests,
            readiness,
            shutdown_transitions_total,
        }
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

    pub fn render(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    #[allow(dead_code)]
    pub fn record_upstream_failure(&self) {
        self.upstream_failures_total.inc();
    }

    #[allow(dead_code)]
    pub fn record_audit_persist_failure(&self) {
        self.audit_persist_failures_total.inc();
    }

    #[allow(dead_code)]
    pub fn record_replay_persist_failure(&self) {
        self.replay_persist_failures_total.inc();
    }

    #[allow(dead_code)]
    pub fn record_shutdown_transition(&self) {
        self.shutdown_transitions_total.inc();
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn test_metrics_snapshot_contains_request_and_readiness_series() {
        let metrics = Metrics::new();
        metrics.set_readiness(true);
        metrics.record_execution("test-route", "POST", "ALLOWED", 0.012);

        let snapshot = metrics.render().unwrap();
        assert!(snapshot.contains("guardrail_requests_total"));
        assert!(snapshot.contains("route_id=\"test-route\""));
        assert!(snapshot.contains("guardrail_readiness 1"));
    }
}

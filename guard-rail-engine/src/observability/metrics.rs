use crate::config::ObservabilityConfig;
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub struct Metrics {
    registry: Registry,
    requests_total: IntCounterVec,
    request_latency_seconds: HistogramVec,
    inflight_requests: IntGauge,
    readiness: IntGauge,
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
            inflight_requests,
            readiness,
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

    pub fn is_ready(&self) -> bool {
        self.readiness.get() > 0
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
}

pub fn attach<S>(
    router: Router<S>,
    config: &ObservabilityConfig,
    metrics: Arc<Metrics>,
    ready: Arc<AtomicBool>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = router.route(
        "/ready",
        get({
            let ready = Arc::clone(&ready);
            move || async move {
                if ready.load(Ordering::Acquire) {
                    (StatusCode::OK, "ready")
                } else {
                    (StatusCode::SERVICE_UNAVAILABLE, "not ready")
                }
            }
        }),
    );

    let router = if config.metrics_enabled {
        router.route(
            &config.metrics_path,
            get({
                let metrics = Arc::clone(&metrics);
                move || {
                    let metrics = Arc::clone(&metrics);
                    async move {
                        match metrics.render() {
                            Ok(snapshot) => (StatusCode::OK, snapshot),
                            Err(err) => (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("metrics render error: {err}"),
                            ),
                        }
                    }
                }
            }),
        )
    } else {
        router
    };

    router.layer(axum::middleware::from_fn_with_state(
        Arc::clone(&metrics),
        record_request_metrics,
    ))
}

pub async fn record_request_metrics(
    State(metrics): State<Arc<Metrics>>,
    request: Request,
    next: Next,
) -> Response {
    let _inflight = metrics.inflight_guard();
    let started = Instant::now();
    let method = request.method().as_str().to_string();
    let route_id = route_id_from_path(request.uri().path());
    let response = next.run(request).await;
    let verdict = verdict_from_status(response.status());
    metrics.record_execution(
        &route_id,
        &method,
        &verdict,
        started.elapsed().as_secs_f64(),
    );
    response
}

fn verdict_from_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED | StatusCode::NO_CONTENT => {
            "ALLOWED"
        }
        StatusCode::FORBIDDEN => "BLOCKED",
        _ => "REJECTED",
    }
}

fn route_id_from_path(path: &str) -> String {
    path.strip_prefix("/v1/execute/")
        .filter(|value| !value.is_empty())
        .unwrap_or(path.trim_start_matches('/'))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request as HttpRequest;
    use axum::routing::post;
    use std::sync::atomic::AtomicBool;
    use tower::ServiceExt;

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

    #[tokio::test]
    async fn test_attach_exposes_metrics_and_records_requests() {
        let metrics = Arc::new(Metrics::new());
        let ready = Arc::new(AtomicBool::new(true));
        let config = ObservabilityConfig {
            service_name: "guard-rail-engine".to_string(),
            metrics_enabled: true,
            metrics_path: "/metrics".to_string(),
            trace_header_name: "traceparent".to_string(),
            readiness_probe_timeout_ms: 250,
        };

        let app = attach(
            Router::new().route("/v1/execute/{route_id}", post(|| async { "ok" })),
            &config,
            Arc::clone(&metrics),
            Arc::clone(&ready),
        );

        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/v1/execute/widget")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let ready_response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready_response.status(), StatusCode::OK);

        let metrics_response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(metrics_response.status(), StatusCode::OK);
        let body = to_bytes(metrics_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot = String::from_utf8(body.to_vec()).unwrap();
        assert!(snapshot.contains("guardrail_requests_total"));
        assert!(snapshot.contains("route_id=\"widget\""));
        assert!(snapshot.contains("verdict=\"ALLOWED\""));
        assert!(!snapshot.contains("verdict=\"200\""));

        ready.store(false, Ordering::Release);
        let ready_response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready_response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

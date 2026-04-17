use crate::config::{LoggingConfig, ObservabilityConfig};
use axum::http::HeaderMap;
use std::error::Error;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(
    logging: &LoggingConfig,
    observability: &ObservabilityConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if logging.format == "pretty" {
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&logging.level)),
            )
            .with(tracing_subscriber::fmt::layer().pretty())
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&logging.level)),
            )
            .with(tracing_subscriber::fmt::layer().json())
            .try_init()?;
    }

    emit_initialized_event(observability);
    Ok(())
}

fn emit_initialized_event(observability: &ObservabilityConfig) {
    tracing::info!(
        service_name = %observability.service_name,
        metrics_enabled = observability.metrics_enabled,
        metrics_path = %observability.metrics_path,
        trace_header_name = %observability.trace_header_name,
        readiness_probe_timeout_ms = observability.readiness_probe_timeout_ms,
        "observability initialized"
    );
}

pub fn trace_id_from_headers(headers: &HeaderMap, trace_header_name: &str) -> String {
    headers
        .get(trace_header_name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(|value| parse_trace_id(value, trace_header_name))
        .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string())
}

fn parse_trace_id(value: &str, trace_header_name: &str) -> String {
    if trace_header_name.eq_ignore_ascii_case("traceparent") {
        let mut parts = value.split('-');
        let _version = parts.next();
        let trace_id = parts.next();
        let span_id = parts.next();
        let flags = parts.next();
        if let (Some(trace_id), Some(_span_id), Some(_flags)) = (trace_id, span_id, flags)
            && trace_id.len() == 32
            && trace_id.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return trace_id.to_string();
        }
    }

    value.to_string()
}

pub fn execution_span(
    trace_id: &str,
    execution_id: &str,
    route_id: &str,
    method: &str,
) -> tracing::Span {
    tracing::info_span!(
        "execution_request",
        trace_id = %trace_id,
        execution_id = %execution_id,
        route_id = %route_id,
        tenant_id = tracing::field::Empty,
        api_key_id = tracing::field::Empty,
        method = %method,
        verdict = tracing::field::Empty,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::Registry;
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

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

        fn record_bool(&mut self, field: &Field, value: bool) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
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
    fn test_emit_initialized_event_uses_observability_fields() {
        let capture = CaptureLayer::default();
        let events = Arc::clone(&capture.events);
        let subscriber = Registry::default().with(capture);
        let observability = ObservabilityConfig {
            service_name: "guard-rail-engine".to_string(),
            metrics_enabled: true,
            metrics_path: "/custom-metrics".to_string(),
            trace_header_name: "x-trace-id".to_string(),
            readiness_probe_timeout_ms: 500,
        };

        tracing::subscriber::with_default(subscriber, || {
            emit_initialized_event(&observability);
        });

        let events = events.lock().expect("capture layer poisoned");
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(
            event.get("service_name"),
            Some(&"guard-rail-engine".to_string())
        );
        assert_eq!(event.get("metrics_enabled"), Some(&"true".to_string()));
        assert_eq!(
            event.get("metrics_path"),
            Some(&"/custom-metrics".to_string())
        );
        assert_eq!(
            event.get("trace_header_name"),
            Some(&"x-trace-id".to_string())
        );
        assert_eq!(
            event.get("readiness_probe_timeout_ms"),
            Some(&"500".to_string())
        );
        assert_eq!(
            event.get("message"),
            Some(&"observability initialized".to_string())
        );
    }

    #[test]
    fn test_trace_id_from_headers_prefers_configured_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
        );

        let trace_id = trace_id_from_headers(&headers, "traceparent");
        assert_eq!(trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
    }

    #[test]
    fn test_trace_id_from_headers_generates_fallback_when_missing() {
        let headers = HeaderMap::new();

        let trace_id = trace_id_from_headers(&headers, "traceparent");
        assert_eq!(trace_id.len(), 32);
        assert!(trace_id.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}

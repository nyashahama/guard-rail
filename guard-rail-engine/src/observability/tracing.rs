use crate::config::ObservabilityConfig;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{Resource, trace::TracerProvider as SdkTracerProvider};
use std::error::Error;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(config: &ObservabilityConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let resource = Resource::new([
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("observability.metrics.enabled", config.metrics_enabled),
        KeyValue::new("observability.metrics.path", config.metrics_path.clone()),
        KeyValue::new(
            "observability.trace_header_name",
            config.trace_header_name.clone(),
        ),
        KeyValue::new(
            "observability.readiness_probe_timeout_ms",
            config.readiness_probe_timeout_ms as i64,
        ),
    ]);

    let provider = SdkTracerProvider::builder().with_resource(resource).build();
    let tracer = provider.tracer(config.service_name.clone());
    global::set_tracer_provider(provider);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().json();
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()?;

    Ok(())
}

use crate::config::{LoggingConfig, ObservabilityConfig};
use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{
    EnvFilter, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn init(
    logging: &LoggingConfig,
    observability: &ObservabilityConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    init_with_writer(logging, observability, io::stdout)
}

pub fn init_with_writer<W>(
    logging: &LoggingConfig,
    observability: &ObservabilityConfig,
    writer: W,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&logging.level));
    if logging.format == "pretty" {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_writer(writer),
            )
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json().with_writer(writer))
            .try_init()?;
    }

    tracing::info!(
        service_name = %observability.service_name,
        metrics_enabled = observability.metrics_enabled,
        metrics_path = %observability.metrics_path,
        trace_header_name = %observability.trace_header_name,
        readiness_probe_timeout_ms = observability.readiness_probe_timeout_ms,
        "observability initialized"
    );

    Ok(())
}

#[derive(Clone, Default)]
pub struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl BufferWriter {
    pub fn new(buffer: Arc<Mutex<Vec<u8>>>) -> Self {
        Self(buffer)
    }

    pub fn buffer(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.0)
    }
}

impl<'a> MakeWriter<'a> for BufferWriter {
    type Writer = BufferWriterGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferWriterGuard(Arc::clone(&self.0))
    }
}

pub struct BufferWriterGuard(Arc<Mutex<Vec<u8>>>);

impl io::Write for BufferWriterGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("buffer writer poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_init_emits_startup_log_with_observability_config() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = BufferWriter::new(Arc::clone(&buffer));
        let logging = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
        };
        let observability = ObservabilityConfig {
            service_name: "guard-rail-engine".to_string(),
            metrics_enabled: true,
            metrics_path: "/custom-metrics".to_string(),
            trace_header_name: "x-trace-id".to_string(),
            readiness_probe_timeout_ms: 500,
        };

        init_with_writer(&logging, &observability, writer).unwrap();
        tracing::info!("startup complete");

        let output = buffer.lock().expect("buffer poisoned").clone();
        let output = str::from_utf8(&output).expect("utf8 output");
        assert!(output.contains("observability initialized"));
        assert!(output.contains("startup complete"));
        assert!(output.contains("guard-rail-engine"));
        assert!(output.contains("/custom-metrics"));
        assert!(output.contains("x-trace-id"));
        assert!(output.contains("500"));
    }
}

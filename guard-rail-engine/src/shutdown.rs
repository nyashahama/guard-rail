use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    #[default]
    Starting,
    Ready,
    Draining,
    Stopped,
}

impl RuntimePhase {
    #[cfg(test)]
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimePhase::Starting => "starting",
            RuntimePhase::Ready => "ready",
            RuntimePhase::Draining => "draining",
            RuntimePhase::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LifecycleState {
    phase: Arc<RwLock<RuntimePhase>>,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleState {
    pub fn new() -> Self {
        Self {
            phase: Arc::new(RwLock::new(RuntimePhase::Starting)),
        }
    }

    pub async fn mark_ready(&self) {
        let mut phase = self.phase.write().await;
        if matches!(*phase, RuntimePhase::Starting | RuntimePhase::Ready) {
            *phase = RuntimePhase::Ready;
        }
    }

    pub async fn begin_drain(&self) {
        *self.phase.write().await = RuntimePhase::Draining;
    }

    pub async fn mark_stopped(&self) {
        *self.phase.write().await = RuntimePhase::Stopped;
    }

    pub async fn current(&self) -> RuntimePhase {
        *self.phase.read().await
    }

    pub async fn is_ready(&self) -> bool {
        matches!(self.current().await, RuntimePhase::Ready)
    }
}

pub async fn shutdown_signal(
    lifecycle: LifecycleState,
    metrics: Option<Arc<crate::observability::metrics::Metrics>>,
) {
    wait_for_signal().await;

    lifecycle.begin_drain().await;
    if let Some(metrics) = &metrics {
        metrics.set_readiness(false);
        metrics.record_shutdown_transition("draining");
    }
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("register SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_lifecycle_state_transitions_affect_readiness() {
        let lifecycle = crate::shutdown::LifecycleState::new();

        assert!(!lifecycle.is_ready().await);

        lifecycle.mark_ready().await;
        assert!(lifecycle.is_ready().await);

        lifecycle.begin_drain().await;
        assert!(!lifecycle.is_ready().await);
        assert_eq!(lifecycle.current().await.as_str(), "draining");
    }

    #[tokio::test]
    async fn test_lifecycle_state_does_not_return_to_ready_after_drain() {
        let lifecycle = crate::shutdown::LifecycleState::new();

        lifecycle.mark_ready().await;
        lifecycle.begin_drain().await;
        lifecycle.mark_ready().await;

        assert!(!lifecycle.is_ready().await);
        assert_eq!(lifecycle.current().await.as_str(), "draining");
    }
}

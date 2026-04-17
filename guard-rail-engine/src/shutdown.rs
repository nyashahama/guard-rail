use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    Starting,
    Ready,
    Draining,
    Stopped,
}

impl RuntimePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimePhase::Starting => "starting",
            RuntimePhase::Ready => "ready",
            RuntimePhase::Draining => "draining",
            RuntimePhase::Stopped => "stopped",
        }
    }
}

impl Default for RuntimePhase {
    fn default() -> Self {
        RuntimePhase::Starting
    }
}

#[derive(Clone, Debug)]
pub struct LifecycleState {
    phase: Arc<RwLock<RuntimePhase>>,
}

impl LifecycleState {
    pub fn new() -> Self {
        Self {
            phase: Arc::new(RwLock::new(RuntimePhase::Starting)),
        }
    }

    pub async fn mark_ready(&self) {
        *self.phase.write().await = RuntimePhase::Ready;
    }

    pub async fn begin_drain(&self) {
        *self.phase.write().await = RuntimePhase::Draining;
    }

    pub async fn current(&self) -> RuntimePhase {
        *self.phase.read().await
    }

    pub async fn is_ready(&self) -> bool {
        matches!(self.current().await, RuntimePhase::Ready)
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
}

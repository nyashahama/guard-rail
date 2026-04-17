use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct TenantRateLimiter {
    inner: Arc<RwLock<HashMap<uuid::Uuid, BucketState>>>,
    requests_per_minute: u32,
    burst: u32,
}

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_refill: std::time::Instant,
}

impl TenantRateLimiter {
    pub fn new(requests_per_minute: u32, burst: u32) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            requests_per_minute,
            burst,
        }
    }

    pub async fn allow(&self, tenant_id: uuid::Uuid) -> bool {
        let mut guard = self.inner.write().await;
        let state = guard.entry(tenant_id).or_insert_with(|| BucketState {
            tokens: self.burst as f64,
            last_refill: std::time::Instant::now(),
        });

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let refill_rate = self.requests_per_minute as f64 / 60.0;
        state.tokens = (state.tokens + elapsed * refill_rate).min(self.burst as f64);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
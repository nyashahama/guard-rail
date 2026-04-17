use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct TenantAuthSnapshot {
    pub route_bindings: HashMap<String, uuid::Uuid>,
    pub api_keys: HashMap<String, CachedApiKey>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CachedApiKey {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub key_prefix: String,
    pub tenant_status: String,
}

#[derive(Clone, Default)]
pub struct TenantAuthCache {
    inner: Arc<RwLock<TenantAuthSnapshot>>,
}

impl TenantAuthCache {
    pub async fn replace(&self, snapshot: TenantAuthSnapshot) {
        *self.inner.write().await = snapshot;
    }

    pub async fn snapshot(&self) -> TenantAuthSnapshot {
        self.inner.read().await.clone()
    }
}

pub fn validate_all_routes_bound(
    routes: &crate::routes::RouteTable,
    snapshot: &TenantAuthSnapshot,
) -> Result<(), String> {
    let unbound: Vec<String> = routes
        .route_ids()
        .into_iter()
        .filter(|route_id| !snapshot.route_bindings.contains_key(route_id))
        .collect();

    if unbound.is_empty() {
        Ok(())
    } else {
        Err(format!("Unbound executable routes: {}", unbound.join(", ")))
    }
}

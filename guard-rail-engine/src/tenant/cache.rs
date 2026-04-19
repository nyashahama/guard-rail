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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAuthStateError {
    PublicRouteBound {
        route_id: String,
        tenant_id: uuid::Uuid,
    },
}

impl std::fmt::Display for RouteAuthStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteAuthStateError::PublicRouteBound {
                route_id,
                tenant_id,
            } => {
                write!(
                    f,
                    "Public route '{}' is bound to tenant {}",
                    route_id, tenant_id
                )
            }
        }
    }
}

pub fn validate_route_auth_state(
    routes: &crate::routes::RouteTable,
    snapshot: &TenantAuthSnapshot,
) -> Result<(), RouteAuthStateError> {
    for route in routes.iter() {
        if matches!(route.auth_mode, crate::routes::RouteAuthMode::Public)
            && let Some(tenant_id) = snapshot.route_bindings.get(&route.id)
        {
            return Err(RouteAuthStateError::PublicRouteBound {
                route_id: route.id.clone(),
                tenant_id: *tenant_id,
            });
        }
    }

    Ok(())
}

#[allow(dead_code)]
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

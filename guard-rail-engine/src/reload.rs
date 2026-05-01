use crate::observability::metrics::Metrics;
use crate::policy::PolicySet;
use crate::routes::RouteTable;
use crate::tenant::cache::TenantAuthCache;
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn start_watcher(
    routes_file: PathBuf,
    policies_dir: PathBuf,
    routes: Arc<RwLock<RouteTable>>,
    policies: Arc<RwLock<PolicySet>>,
    tenant_cache: TenantAuthCache,
    environment: crate::config::RuntimeEnvironment,
    metrics: Option<Arc<Metrics>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Handle::current();
    let rt_initial = rt.clone();

    let routes_path = routes_file.clone();
    let policies_path = policies_dir.clone();
    let initial_tenant_cache = tenant_cache.clone();
    let initial_metrics = metrics.clone();

    let callback_routes = Arc::clone(&routes);
    let callback_policies = Arc::clone(&policies);
    let callback_tenant_cache = tenant_cache.clone();
    let callback_routes_path = routes_file.clone();
    let callback_policies_path = policies_dir.clone();
    let callback_environment = environment;
    let callback_metrics = metrics;

    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res
                && matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                )
            {
                let routes = Arc::clone(&callback_routes);
                let policies = Arc::clone(&callback_policies);
                let tenant_cache = callback_tenant_cache.clone();
                let routes_path = callback_routes_path.clone();
                let policies_path = callback_policies_path.clone();
                let environment = callback_environment;
                let metrics = callback_metrics.clone();

                rt.spawn(async move {
                    reload_all(
                        &routes_path,
                        &policies_path,
                        &routes,
                        &policies,
                        &tenant_cache,
                        environment,
                        metrics,
                    )
                    .await;
                });
            }
        })?;

    if let Some(parent) = routes_file.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
    }
    watcher.watch(&policies_dir, RecursiveMode::Recursive)?;

    std::mem::forget(watcher);

    let final_routes = Arc::clone(&routes);
    let final_policies = Arc::clone(&policies);
    let final_metrics = initial_metrics;
    rt_initial.spawn(async move {
        reload_all(
            &routes_path,
            &policies_path,
            &final_routes,
            &final_policies,
            &initial_tenant_cache,
            environment,
            final_metrics,
        )
        .await;
    });

    tracing::info!("File watcher started for routes and policies");
    Ok(())
}

pub async fn apply_reload_candidate(
    new_routes: RouteTable,
    new_policies: PolicySet,
    routes: &Arc<RwLock<RouteTable>>,
    policies: &Arc<RwLock<PolicySet>>,
    tenant_cache: &TenantAuthCache,
    environment: crate::config::RuntimeEnvironment,
) -> Result<(), String> {
    let required = new_routes.policy_names();
    new_policies
        .validate_references(&required)
        .map_err(|err| err.to_string())?;

    let snapshot = tenant_cache.snapshot().await;
    crate::tenant::cache::validate_route_auth_state(&new_routes, &snapshot)
        .map_err(|err| format!("{err:?}"))?;

    crate::routes::RouteValidator::validate_upstream_security(&new_routes, environment)
        .map_err(|err| err.to_string())?;

    *routes.write().await = new_routes;
    *policies.write().await = new_policies;
    Ok(())
}

async fn reload_all(
    routes_path: &Path,
    policies_path: &Path,
    routes: &Arc<RwLock<RouteTable>>,
    policies: &Arc<RwLock<PolicySet>>,
    tenant_cache: &TenantAuthCache,
    environment: crate::config::RuntimeEnvironment,
    metrics: Option<Arc<Metrics>>,
) {
    if let Some(metrics) = &metrics {
        metrics.record_reload_event("started");
    }

    let new_policies = match PolicySet::load_dir(policies_path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Policies reload failed, keeping previous config: {}", e);
            if let Some(metrics) = &metrics {
                metrics.record_reload_event("failed");
            }
            return;
        }
    };

    let new_routes = match RouteTable::load(routes_path) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Routes reload failed, keeping previous config: {}", e);
            if let Some(metrics) = &metrics {
                metrics.record_reload_event("failed");
            }
            return;
        }
    };

    if let Err(e) = apply_reload_candidate(
        new_routes,
        new_policies,
        routes,
        policies,
        tenant_cache,
        environment,
    )
    .await
    {
        tracing::warn!("Reload rejected — {}", e);
        if let Some(metrics) = &metrics {
            metrics.record_reload_event("rejected");
        }
        return;
    }

    if let Some(metrics) = &metrics {
        metrics.record_reload_event("succeeded");
        metrics.record_reload_success_now();
    }

    tracing::info!("Routes and policies reloaded successfully");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_apply_reload_candidate_allows_unbound_tenant_bound_route() {
        let routes = std::sync::Arc::new(tokio::sync::RwLock::new(RouteTable::from_routes(vec![])));
        let policies =
            std::sync::Arc::new(tokio::sync::RwLock::new(PolicySet::from_policies(vec![])));
        let tenant_cache = crate::tenant::cache::TenantAuthCache::default();

        let new_routes = RouteTable::from_routes(vec![crate::routes::Route::new(
            "tenant-route".into(),
            crate::routes::RouteAuthMode::TenantBound,
            "http://example.com".into(),
            vec!["POST".into()],
            vec![],
        )]);

        let result = apply_reload_candidate(
            new_routes,
            PolicySet::from_policies(vec![]),
            &routes,
            &policies,
            &tenant_cache,
            crate::config::RuntimeEnvironment::Development,
        )
        .await;

        assert!(result.is_ok());
        assert!(routes.read().await.lookup("tenant-route").is_some());
    }

    #[tokio::test]
    async fn test_apply_reload_candidate_rejects_bound_public_route() {
        let routes = std::sync::Arc::new(tokio::sync::RwLock::new(RouteTable::from_routes(vec![])));
        let policies =
            std::sync::Arc::new(tokio::sync::RwLock::new(PolicySet::from_policies(vec![])));
        let tenant_cache = crate::tenant::cache::TenantAuthCache::default();
        let tenant_id = uuid::Uuid::new_v4();
        let mut snapshot = crate::tenant::cache::TenantAuthSnapshot::default();
        snapshot
            .route_bindings
            .insert("open-route".into(), tenant_id);
        tenant_cache.replace(snapshot).await;

        let new_routes = RouteTable::from_routes(vec![crate::routes::Route::new(
            "open-route".into(),
            crate::routes::RouteAuthMode::Public,
            "http://example.com".into(),
            vec!["POST".into()],
            vec![],
        )]);

        let result = apply_reload_candidate(
            new_routes,
            PolicySet::from_policies(vec![]),
            &routes,
            &policies,
            &tenant_cache,
            crate::config::RuntimeEnvironment::Development,
        )
        .await;

        assert!(result.is_err());
        assert!(routes.read().await.lookup("open-route").is_none());
    }

    #[tokio::test]
    async fn test_apply_reload_candidate_keeps_last_good_routes_on_rejection() {
        let original_routes = RouteTable::from_routes(vec![crate::routes::Route::new(
            "open-route".into(),
            crate::routes::RouteAuthMode::Public,
            "http://example.com/original".into(),
            vec!["POST".into()],
            vec![],
        )]);
        let routes = std::sync::Arc::new(tokio::sync::RwLock::new(original_routes));
        let policies =
            std::sync::Arc::new(tokio::sync::RwLock::new(PolicySet::from_policies(vec![])));
        let tenant_cache = crate::tenant::cache::TenantAuthCache::default();

        let rejected_routes = RouteTable::from_routes(vec![crate::routes::Route::new(
            "tenant-route".into(),
            crate::routes::RouteAuthMode::TenantBound,
            "http://example.com/rejected".into(),
            vec!["POST".into()],
            vec!["missing-policy".into()],
        )]);

        let result = apply_reload_candidate(
            rejected_routes,
            PolicySet::from_policies(vec![]),
            &routes,
            &policies,
            &tenant_cache,
            crate::config::RuntimeEnvironment::Development,
        )
        .await;

        assert!(result.is_err());
        let retained = routes.read().await;
        assert!(retained.lookup("open-route").is_some());
        assert!(retained.lookup("tenant-route").is_none());
    }
}

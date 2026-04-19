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
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Handle::current();
    let rt_initial = rt.clone();

    let routes_path = routes_file.clone();
    let policies_path = policies_dir.clone();
    let initial_tenant_cache = tenant_cache.clone();

    let callback_routes = Arc::clone(&routes);
    let callback_policies = Arc::clone(&policies);
    let callback_tenant_cache = tenant_cache.clone();
    let callback_routes_path = routes_file.clone();
    let callback_policies_path = policies_dir.clone();
    let callback_environment = environment;

    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
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

            rt.spawn(async move {
                reload_all(&routes_path, &policies_path, &routes, &policies, &tenant_cache, environment).await;
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
    rt_initial.spawn(async move {
        reload_all(
            &routes_path,
            &policies_path,
            &final_routes,
            &final_policies,
            &initial_tenant_cache,
            environment,
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
        .map_err(|err| format!("{err}"))?;

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
) {
    let new_policies = match PolicySet::load_dir(policies_path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Policies reload failed, keeping previous config: {}", e);
            return;
        }
    };

    let new_routes = match RouteTable::load(routes_path) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Routes reload failed, keeping previous config: {}", e);
            return;
        }
    };

    if let Err(e) =
        apply_reload_candidate(new_routes, new_policies, routes, policies, tenant_cache, environment).await
    {
        tracing::warn!("Reload rejected — {}", e);
        return;
    }

    tracing::info!("Routes and policies reloaded successfully");
}

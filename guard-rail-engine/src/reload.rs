use crate::policy::PolicySet;
use crate::routes::RouteTable;
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn start_watcher(
    routes_file: PathBuf,
    policies_dir: PathBuf,
    routes: Arc<RwLock<RouteTable>>,
    policies: Arc<RwLock<PolicySet>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Handle::current();

    let routes_path = routes_file.clone();
    let policies_path = policies_dir.clone();

    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        let routes = Arc::clone(&routes);
                        let policies = Arc::clone(&policies);
                        let routes_path = routes_path.clone();
                        let policies_path = policies_path.clone();

                        rt.spawn(async move {
                            reload_all(&routes_path, &policies_path, &routes, &policies).await;
                        });
                    }
                    _ => {}
                }
            }
        })?;

    if let Some(parent) = routes_file.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
    }

    watcher.watch(&policies_dir, RecursiveMode::Recursive)?;

    // Leak the watcher so it lives for the duration of the process
    std::mem::forget(watcher);

    tracing::info!("File watcher started for routes and policies");
    Ok(())
}

async fn reload_all(
    routes_path: &PathBuf,
    policies_path: &PathBuf,
    routes: &Arc<RwLock<RouteTable>>,
    policies: &Arc<RwLock<PolicySet>>,
) {
    // Load policies and routes synchronously, extracting results before any .await
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

    let required = new_routes.policy_names();
    if let Err(e) = new_policies.validate_references(&required) {
        tracing::warn!("Reload rejected — {}", e);
        return;
    }

    *routes.write().await = new_routes;
    *policies.write().await = new_policies;
    tracing::info!("Routes and policies reloaded successfully");
}

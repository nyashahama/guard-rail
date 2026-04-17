use crate::audit::hash::hash_string;
use crate::policy::Policy;
use crate::routes::Route;
use axum::http::HeaderMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicySnapshotRecord {
    pub snapshot_hash: String,
    pub route_id: String,
    pub route_definition: serde_json::Value,
    pub policies_definition: serde_json::Value,
    pub route_config_hash: String,
    pub policy_set_hash: String,
}

pub fn build_snapshot(route: &Route, policies: &[Policy]) -> PolicySnapshotRecord {
    let route_definition = serde_json::json!({
        "id": route.id,
        "path": route.path,
        "upstream": route.upstream,
        "methods": route.methods,
        "policies": route.policies,
        "timeout_ms": route.timeout_ms,
    });

    let policies_definition = serde_json::to_value(policies).unwrap();
    let route_config_hash = hash_string(&route_definition.to_string());
    let policy_set_hash = hash_string(&policies_definition.to_string());
    let snapshot_hash = hash_string(
        &serde_json::json!({
            "route": route_definition,
            "policies": policies_definition,
        })
        .to_string(),
    );

    PolicySnapshotRecord {
        snapshot_hash,
        route_id: route.id.clone(),
        route_definition,
        policies_definition,
        route_config_hash,
        policy_set_hash,
    }
}

impl PolicySnapshotRecord {
    pub fn from_route_and_set(
        route: &Route,
        policy_set: &crate::policy::PolicySet,
    ) -> Result<Self, String> {
        let policies = policy_set.policies_for_route(route)?;
        Ok(build_snapshot(route, &policies))
    }
}

pub fn build_snapshot_from_set(
    route: &Route,
    policy_set: &crate::policy::PolicySet,
) -> Result<PolicySnapshotRecord, String> {
    PolicySnapshotRecord::from_route_and_set(route, policy_set)
}

pub fn filter_headers(headers: &HeaderMap, capture_list: &[String]) -> serde_json::Value {
    let mut result = serde_json::Map::new();
    for key in capture_list {
        if let Some(value) = headers.get(key.as_str())
            && let Ok(v) = value.to_str()
        {
            result.insert(key.clone(), serde_json::Value::String(v.to_string()));
        }
    }
    if result.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::Value::Object(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_hash_is_stable_for_equivalent_route_and_policy_state() {
        let route = Route {
            id: "payments".into(),
            path: "/v1/execute/payments".into(),
            upstream: "http://upstream/payments".into(),
            methods: vec!["POST".into()],
            policies: vec!["block-callbacks".into()],
            timeout_ms: 5000,
        };

        let snapshot_a = build_snapshot(
            &route,
            &[Policy {
                name: "block-callbacks".into(),
                description: "".into(),
                rules: vec![],
            }],
        );

        let snapshot_b = build_snapshot(
            &route,
            &[Policy {
                name: "block-callbacks".into(),
                description: "".into(),
                rules: vec![],
            }],
        );

        assert_eq!(snapshot_a.snapshot_hash, snapshot_b.snapshot_hash);
    }

    #[test]
    fn test_snapshot_builder_uses_only_route_referenced_policies() {
        let route = Route {
            id: "payments".into(),
            path: "/v1/execute/payments".into(),
            upstream: "http://upstream/payments".into(),
            methods: vec!["POST".into()],
            policies: vec!["policy-a".into()],
            timeout_ms: 5000,
        };

        let set = crate::policy::PolicySet::from_policies(vec![
            Policy {
                name: "policy-a".into(),
                description: "".into(),
                rules: vec![],
            },
            Policy {
                name: "policy-b".into(),
                description: "".into(),
                rules: vec![],
            },
        ]);

        let snapshot = build_snapshot_from_set(&route, &set).unwrap();
        assert_eq!(snapshot.policies_definition.as_array().unwrap().len(), 1);
    }
}

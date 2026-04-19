use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteAuthMode {
    Public,
    TenantBound,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutesConfig {
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Route {
    pub id: String,
    pub auth_mode: RouteAuthMode,
    pub upstream: String,
    pub methods: Vec<String>,
    pub policies: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone)]
pub struct RouteTable {
    by_id: std::collections::HashMap<String, Route>,
}

impl RouteTable {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: RoutesConfig = serde_yaml::from_str(&contents)?;

        let mut by_id = std::collections::HashMap::new();
        for route in config.routes {
            if by_id.contains_key(&route.id) {
                return Err(format!("Duplicate route id: {}", route.id).into());
            }
            by_id.insert(route.id.clone(), route);
        }

        Ok(RouteTable { by_id })
    }

    pub fn lookup(&self, route_id: &str) -> Option<&Route> {
        self.by_id.get(route_id)
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &Route> {
        self.by_id.values()
    }

    pub fn policy_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .by_id
            .values()
            .flat_map(|r| r.policies.iter().cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    #[allow(dead_code)]
    pub fn from_routes(routes: Vec<Route>) -> Self {
        let by_id = routes
            .into_iter()
            .map(|route| (route.id.clone(), route))
            .collect();
        Self { by_id }
    }

    pub fn route_ids(&self) -> Vec<String> {
        let mut ids = self.by_id.keys().cloned().collect::<Vec<_>>();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteAuthMode;
    use std::io::Write;

    fn write_yaml(yaml: &str) -> tempfile::NamedTempFile {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();
        tmp
    }

    #[test]
    fn test_missing_auth_mode_errors() {
        let tmp = write_yaml(
            r#"
routes:
  - id: missing-auth-mode
    path: /v1/execute/missing-auth-mode
    upstream: https://example.com
    methods: [POST]
    policies: []
"#,
        );

        let err = RouteTable::load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("auth_mode"));
    }

    #[test]
    fn test_validate_route_auth_state_allows_unbound_tenant_bound_route() {
        let routes = RouteTable::from_routes(vec![Route {
            id: "bootstrap-route".into(),
            auth_mode: RouteAuthMode::TenantBound,
            upstream: "http://upstream".into(),
            methods: vec!["POST".into()],
            policies: vec![],
            timeout_ms: 5000,
        }]);

        let snapshot = crate::tenant::cache::TenantAuthSnapshot::default();
        assert!(crate::tenant::cache::validate_route_auth_state(&routes, &snapshot).is_ok());
    }

    #[test]
    fn test_validate_route_auth_state_rejects_bound_public_route() {
        let routes = RouteTable::from_routes(vec![Route {
            id: "open-route".into(),
            auth_mode: RouteAuthMode::Public,
            upstream: "http://upstream".into(),
            methods: vec!["POST".into()],
            policies: vec![],
            timeout_ms: 5000,
        }]);

        let mut snapshot = crate::tenant::cache::TenantAuthSnapshot::default();
        snapshot
            .route_bindings
            .insert("open-route".into(), uuid::Uuid::nil());

        let err = crate::tenant::cache::validate_route_auth_state(&routes, &snapshot).unwrap_err();
        assert!(matches!(
            err,
            crate::tenant::cache::RouteAuthStateError::PublicRouteBound { .. }
        ));
    }

    #[test]
    fn test_load_valid_routes() {
        let tmp = write_yaml(
            r#"
routes:
  - id: transfer-api
    path: /v1/execute/transfer-api
    auth_mode: public
    upstream: https://bank.za/api/transfer
    methods: [POST, PUT]
    policies: [block-callbacks]
    timeout_ms: 3000
  - id: partner
    path: /v1/execute/partner
    auth_mode: tenant_bound
    upstream: https://erp.internal/webhook
    methods: [POST]
    policies: [size-limit]
"#,
        );

        let table = RouteTable::load(tmp.path()).unwrap();
        assert!(table.lookup("transfer-api").is_some());
        assert!(table.lookup("partner").is_some());
        assert!(table.lookup("nonexistent").is_none());

        let route = table.lookup("transfer-api").unwrap();
        assert_eq!(route.upstream, "https://bank.za/api/transfer");
        assert_eq!(route.methods, vec!["POST", "PUT"]);
        assert_eq!(route.timeout_ms, 3000);
    }

    #[test]
    fn test_default_timeout() {
        let tmp = write_yaml(
            r#"
routes:
  - id: test
    path: /v1/execute/test
    auth_mode: public
    upstream: https://example.com
    methods: [POST]
    policies: []
"#,
        );

        let table = RouteTable::load(tmp.path()).unwrap();
        assert_eq!(table.lookup("test").unwrap().timeout_ms, 5000);
    }

    #[test]
    fn test_duplicate_route_id_errors() {
        let tmp = write_yaml(
            r#"
routes:
  - id: dup
    path: /v1/execute/dup
    auth_mode: public
    upstream: https://a.com
    methods: [POST]
    policies: []
  - id: dup
    path: /v1/execute/dup2
    auth_mode: public
    upstream: https://b.com
    methods: [POST]
    policies: []
"#,
        );

        let result = RouteTable::load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_names_extraction() {
        let tmp = write_yaml(
            r#"
routes:
  - id: a
    path: /v1/execute/a
    auth_mode: public
    upstream: https://a.com
    methods: [POST]
    policies: [pol-a, pol-b]
  - id: b
    path: /v1/execute/b
    auth_mode: tenant_bound
    upstream: https://b.com
    methods: [POST]
    policies: [pol-b, pol-c]
"#,
        );

        let table = RouteTable::load(tmp.path()).unwrap();
        assert_eq!(table.policy_names(), vec!["pol-a", "pol-b", "pol-c"]);
    }

    #[test]
    fn test_method_check() {
        let tmp = write_yaml(
            r#"
routes:
  - id: test
    path: /v1/execute/test
    auth_mode: public
    upstream: https://example.com
    methods: [POST]
    policies: []
"#,
        );

        let table = RouteTable::load(tmp.path()).unwrap();
        let route = table.lookup("test").unwrap();
        assert!(route.methods.contains(&"POST".to_string()));
        assert!(!route.methods.contains(&"GET".to_string()));
    }

    #[test]
    fn test_validate_all_routes_bound_returns_error_for_unbound_route() {
        let routes = RouteTable::from_routes(vec![Route {
            id: "test-route".to_string(),
            auth_mode: RouteAuthMode::TenantBound,
            upstream: "http://upstream".to_string(),
            methods: vec!["POST".to_string()],
            policies: vec![],
            timeout_ms: 5000,
        }]);

        let snapshot = crate::tenant::cache::TenantAuthSnapshot::default();
        let err = crate::tenant::cache::validate_all_routes_bound(&routes, &snapshot).unwrap_err();
        assert!(err.contains("test-route"));
    }
}

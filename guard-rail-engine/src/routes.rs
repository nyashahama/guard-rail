use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct RoutesConfig {
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub id: String,
    #[allow(dead_code)]
    pub path: String,
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
    by_id: HashMap<String, Route>,
}

impl RouteTable {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: RoutesConfig = serde_yaml::from_str(&contents)?;

        let mut by_id = HashMap::new();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_yaml(yaml: &str) -> tempfile::NamedTempFile {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();
        tmp
    }

    #[test]
    fn test_load_valid_routes() {
        let tmp = write_yaml(
            r#"
routes:
  - id: transfer-api
    path: /v1/execute/transfer-api
    upstream: https://bank.za/api/transfer
    methods: [POST, PUT]
    policies: [block-callbacks]
    timeout_ms: 3000
  - id: partner
    path: /v1/execute/partner
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
    upstream: https://a.com
    methods: [POST]
    policies: []
  - id: dup
    path: /v1/execute/dup2
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
    upstream: https://a.com
    methods: [POST]
    policies: [pol-a, pol-b]
  - id: b
    path: /v1/execute/b
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
}

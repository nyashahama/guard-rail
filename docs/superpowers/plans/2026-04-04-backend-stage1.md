# Guard Rail Backend Stage 1: Core Proxy + Policy Engine — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an HTTP API gateway in Rust that receives webhook/API payloads, evaluates them against YAML-defined policies using JSONPath field inspection, and either forwards allowed requests upstream or blocks them.

**Architecture:** Single async Rust binary using axum. One primary endpoint `POST /v1/execute/{route_id}`. Routes map to upstreams + policies. Policy engine inspects JSON payloads via JSONPath, evaluates conditions, returns ALLOW/BLOCK verdicts. Allowed requests forwarded via reqwest. Everything logged as structured JSON to stdout.

**Tech Stack:** Rust, axum, tokio, reqwest, serde/serde_yaml/serde_json, jsonpath-rust, regex, notify, tracing, uuid, tower-http, clap, chrono, url

---

## File Structure

```
guard-rail-engine/
  Cargo.toml
  src/
    main.rs              — entry point, CLI parsing, server startup
    config.rs            — config.yaml loading + env var overrides
    routes.rs            — routes.yaml loading, route matching
    policy/
      mod.rs             — policy types, loading from YAML, policy set management
      engine.rs          — evaluation logic (match policies to payload, return verdict)
      conditions.rs      — condition implementations (domain_check, regex, equals, etc.)
    proxy/
      mod.rs             — request handler, forwarding orchestration
      forward.rs         — upstream HTTP forwarding, header manipulation
      response.rs        — response construction (block, reject, bad gateway)
    logging.rs           — execution log struct, JSON serialization to stdout
    reload.rs            — file watcher setup, hot-reload coordination
  config/
    config.yaml          — example server config
    routes.yaml          — example routes
    policies/
      security.yaml      — example security policies
  tests/
    integration_test.rs  — full request cycle integration tests
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `guard-rail-engine/Cargo.toml`
- Create: `guard-rail-engine/src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "guard-rail-engine"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
jsonpath-rust = "0.7"
regex = "1"
notify = { version = "7", features = ["macos_fsevent"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
uuid = { version = "1", features = ["v4"] }
tower-http = { version = "0.6", features = ["limit"] }
clap = { version = "4", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
url = "2"
glob-match = "0.2"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

- [ ] **Step 2: Create minimal main.rs**

```rust
use axum::{Router, routing::get};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Guard Rail Engine starting on :8080");
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add guard-rail-engine/Cargo.toml guard-rail-engine/src/main.rs
git commit -m "scaffold guard-rail-engine Rust project with dependencies"
```

---

### Task 2: Config Loading

**Files:**
- Create: `guard-rail-engine/src/config.rs`
- Create: `guard-rail-engine/config/config.yaml`

- [ ] **Step 1: Create config.rs with types and loading**

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub routes_file: String,
    pub policies_dir: String,
    pub forwarding: ForwardingConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_body_limit")]
    pub request_body_limit_bytes: usize,
}

fn default_body_limit() -> usize {
    1_048_576
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForwardingConfig {
    #[serde(default = "default_timeout")]
    pub default_timeout_ms: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

fn default_timeout() -> u64 {
    5000
}

fn default_user_agent() -> String {
    "GuardRail/0.1.0".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "json".to_string()
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_yaml::from_str(&contents)?;

        // Environment variable overrides
        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__HOST") {
            config.server.host = val;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_SERVER__PORT") {
            config.server.port = val.parse()?;
        }
        if let Ok(val) = std::env::var("GUARDRAIL_LOGGING__LEVEL") {
            config.logging.level = val;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_valid_config() {
        let yaml = r#"
server:
  host: "127.0.0.1"
  port: 9090
  request_body_limit_bytes: 512000
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding:
  default_timeout_ms: 3000
  user_agent: "TestAgent/1.0"
logging:
  level: "debug"
  format: "pretty"
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.request_body_limit_bytes, 512000);
        assert_eq!(config.forwarding.default_timeout_ms, 3000);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_load_config_with_defaults() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 8080
routes_file: "./routes.yaml"
policies_dir: "./policies/"
forwarding: {}
logging: {}
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.server.request_body_limit_bytes, 1_048_576);
        assert_eq!(config.forwarding.default_timeout_ms, 5000);
        assert_eq!(config.forwarding.user_agent, "GuardRail/0.1.0");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "json");
    }

    #[test]
    fn test_load_invalid_yaml_errors() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"not: [valid: yaml: {{").unwrap();

        let result = AppConfig::load(tmp.path());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Add tempfile dev dependency to Cargo.toml**

Add under `[dev-dependencies]`:
```toml
tempfile = "3"
```

- [ ] **Step 3: Create example config file**

Create `guard-rail-engine/config/config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  request_body_limit_bytes: 1048576

routes_file: "./config/routes.yaml"
policies_dir: "./config/policies/"

forwarding:
  default_timeout_ms: 5000
  user_agent: "GuardRail/0.1.0"

logging:
  level: "info"
  format: "json"
```

- [ ] **Step 4: Add mod to main.rs**

Update `guard-rail-engine/src/main.rs` — add at the top:
```rust
mod config;
```

- [ ] **Step 5: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test config 2>&1 | tail -15`
Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add guard-rail-engine/src/config.rs guard-rail-engine/src/main.rs guard-rail-engine/config/config.yaml guard-rail-engine/Cargo.toml
git commit -m "add config loading with YAML parsing, env var overrides, and defaults"
```

---

### Task 3: Route Loading

**Files:**
- Create: `guard-rail-engine/src/routes.rs`
- Create: `guard-rail-engine/config/routes.yaml`

- [ ] **Step 1: Create routes.rs with types, loading, and lookup**

```rust
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
```

- [ ] **Step 2: Create example routes.yaml**

Create `guard-rail-engine/config/routes.yaml`:

```yaml
routes:
  - id: transfer-api
    path: /v1/execute/transfer-api
    upstream: https://internal-bank.za/api/v1/transfer
    methods: [POST, PUT]
    policies: [block-external-callbacks, pii-detection]
    timeout_ms: 5000

  - id: partner-webhook
    path: /v1/execute/partner-webhook
    upstream: https://erp.internal/webhooks/partner
    methods: [POST]
    policies: [payload-size-limit]
    timeout_ms: 3000
```

- [ ] **Step 3: Add mod to main.rs**

Add to the top of `guard-rail-engine/src/main.rs`:
```rust
mod routes;
```

- [ ] **Step 4: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test routes 2>&1 | tail -15`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add guard-rail-engine/src/routes.rs guard-rail-engine/src/main.rs guard-rail-engine/config/routes.yaml
git commit -m "add route loading with YAML parsing and route lookup"
```

---

### Task 4: Policy Types & Loading

**Files:**
- Create: `guard-rail-engine/src/policy/mod.rs`
- Create: `guard-rail-engine/config/policies/security.yaml`

- [ ] **Step 1: Create policy/mod.rs with types and loading**

```rust
pub mod conditions;
pub mod engine;

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PoliciesFile {
    pub policies: Vec<Policy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub field: String,
    pub condition: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub max_bytes: Option<usize>,
    pub action: String,
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "warning".to_string()
}

#[derive(Debug, Clone)]
pub struct PolicySet {
    by_name: HashMap<String, Policy>,
}

impl PolicySet {
    pub fn load_dir(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut by_name = HashMap::new();

        if !dir.is_dir() {
            return Err(format!("Policies directory not found: {}", dir.display()).into());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                let contents = std::fs::read_to_string(&path)?;
                let file: PoliciesFile = serde_yaml::from_str(&contents).map_err(|e| {
                    format!("Failed to parse {}: {}", path.display(), e)
                })?;
                for policy in file.policies {
                    if by_name.contains_key(&policy.name) {
                        return Err(
                            format!("Duplicate policy name: {}", policy.name).into()
                        );
                    }
                    by_name.insert(policy.name.clone(), policy);
                }
            }
        }

        Ok(PolicySet { by_name })
    }

    pub fn get(&self, name: &str) -> Option<&Policy> {
        self.by_name.get(name)
    }

    pub fn validate_references(&self, required: &[String]) -> Result<(), String> {
        for name in required {
            if !self.by_name.contains_key(name) {
                return Err(format!("Route references unknown policy: {}", name));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_policy(dir: &Path, filename: &str, yaml: &str) {
        fs::write(dir.join(filename), yaml).unwrap();
    }

    #[test]
    fn test_load_policies_from_dir() {
        let dir = TempDir::new().unwrap();
        write_policy(
            dir.path(),
            "security.yaml",
            r#"
policies:
  - name: block-callbacks
    description: Block external callbacks
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.safe.com"]
        action: block
        severity: critical
  - name: size-limit
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 1024
        action: block
"#,
        );

        let set = PolicySet::load_dir(dir.path()).unwrap();
        assert!(set.get("block-callbacks").is_some());
        assert!(set.get("size-limit").is_some());
        assert!(set.get("nonexistent").is_none());

        let policy = set.get("block-callbacks").unwrap();
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.rules[0].condition, "domain_not_in");
    }

    #[test]
    fn test_load_multiple_files() {
        let dir = TempDir::new().unwrap();
        write_policy(
            dir.path(),
            "a.yaml",
            r#"
policies:
  - name: pol-a
    rules:
      - field: "$.x"
        condition: equals
        value: "bad"
        action: block
"#,
        );
        write_policy(
            dir.path(),
            "b.yaml",
            r#"
policies:
  - name: pol-b
    rules:
      - field: "$.y"
        condition: contains
        value: "evil"
        action: block
"#,
        );

        let set = PolicySet::load_dir(dir.path()).unwrap();
        assert!(set.get("pol-a").is_some());
        assert!(set.get("pol-b").is_some());
    }

    #[test]
    fn test_duplicate_policy_name_errors() {
        let dir = TempDir::new().unwrap();
        write_policy(
            dir.path(),
            "a.yaml",
            r#"
policies:
  - name: dup
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 100
        action: block
"#,
        );
        write_policy(
            dir.path(),
            "b.yaml",
            r#"
policies:
  - name: dup
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 200
        action: block
"#,
        );

        let result = PolicySet::load_dir(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_references() {
        let dir = TempDir::new().unwrap();
        write_policy(
            dir.path(),
            "a.yaml",
            r#"
policies:
  - name: exists
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 100
        action: block
"#,
        );

        let set = PolicySet::load_dir(dir.path()).unwrap();
        assert!(set
            .validate_references(&["exists".to_string()])
            .is_ok());
        assert!(set
            .validate_references(&["missing".to_string()])
            .is_err());
    }

    #[test]
    fn test_invalid_yaml_errors() {
        let dir = TempDir::new().unwrap();
        write_policy(dir.path(), "bad.yaml", "not: [valid: yaml: {{");

        let result = PolicySet::load_dir(dir.path());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Create example policies file**

Create `guard-rail-engine/config/policies/security.yaml`:

```yaml
policies:
  - name: block-external-callbacks
    description: Block payloads with callback URLs outside allowlist
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.guardrail.co.za", "*.internal.bank.za"]
        action: block
        severity: critical

  - name: pii-detection
    description: Block payloads containing South African ID numbers
    rules:
      - field: "$..*.value"
        condition: regex_match
        pattern: "\\b\\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\\d|3[01])\\d{4}[01]\\d{2}\\b"
        action: block
        severity: critical

  - name: payload-size-limit
    description: Reject oversized payloads
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 102400
        action: block
        severity: warning
```

- [ ] **Step 3: Create placeholder files for submodules**

Create `guard-rail-engine/src/policy/engine.rs`:
```rust
// Policy evaluation engine — implemented in Task 6
```

Create `guard-rail-engine/src/policy/conditions.rs`:
```rust
// Condition implementations — implemented in Task 5
```

- [ ] **Step 4: Add mod to main.rs**

Add to the top of `guard-rail-engine/src/main.rs`:
```rust
mod policy;
```

- [ ] **Step 5: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test policy:: 2>&1 | tail -15`
Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add guard-rail-engine/src/policy/ guard-rail-engine/config/policies/
git commit -m "add policy types, YAML loading, and policy set management"
```

---

### Task 5: Policy Conditions

**Files:**
- Modify: `guard-rail-engine/src/policy/conditions.rs`

Implements all 11 condition types from the spec.

- [ ] **Step 1: Implement all conditions**

Replace `guard-rail-engine/src/policy/conditions.rs` with:

```rust
use regex::Regex;
use url::Url;

/// Result of evaluating a condition. `true` means the condition triggered (should block).
pub fn evaluate_condition(
    condition: &str,
    extracted_values: &[serde_json::Value],
    params: &ConditionParams,
    raw_payload_bytes: usize,
) -> Result<bool, String> {
    match condition {
        "domain_not_in" => domain_not_in(extracted_values, &params.values),
        "domain_in" => domain_in(extracted_values, &params.values),
        "regex_match" => regex_match(extracted_values, params.pattern.as_deref()),
        "regex_not_match" => regex_not_match(extracted_values, params.pattern.as_deref()),
        "equals" => equals(extracted_values, &params.value),
        "not_equals" => not_equals(extracted_values, &params.value),
        "contains" => contains_check(extracted_values, &params.value),
        "not_contains" => not_contains_check(extracted_values, &params.value),
        "size_exceeds" => size_exceeds(raw_payload_bytes, params.max_bytes),
        "field_exists" => Ok(!extracted_values.is_empty()),
        "field_not_exists" => Ok(extracted_values.is_empty()),
        _ => Err(format!("Unknown condition: {}", condition)),
    }
}

#[derive(Debug, Default)]
pub struct ConditionParams {
    pub values: Vec<String>,
    pub value: Option<serde_json::Value>,
    pub pattern: Option<String>,
    pub max_bytes: Option<usize>,
}

fn domain_matches_glob(domain: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[1..]; // ".example.com"
        domain.ends_with(suffix) || domain == &pattern[2..]
    } else {
        domain == pattern
    }
}

fn extract_domain(val: &serde_json::Value) -> Option<String> {
    let s = val.as_str()?;
    let url = Url::parse(s).ok()?;
    url.host_str().map(|h| h.to_lowercase())
}

fn domain_not_in(values: &[serde_json::Value], allowlist: &[String]) -> Result<bool, String> {
    for val in values {
        if let Some(domain) = extract_domain(val) {
            let allowed = allowlist.iter().any(|pat| domain_matches_glob(&domain, pat));
            if !allowed {
                return Ok(true); // triggers block
            }
        }
    }
    Ok(false)
}

fn domain_in(values: &[serde_json::Value], blocklist: &[String]) -> Result<bool, String> {
    for val in values {
        if let Some(domain) = extract_domain(val) {
            let blocked = blocklist.iter().any(|pat| domain_matches_glob(&domain, pat));
            if blocked {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn regex_match(values: &[serde_json::Value], pattern: Option<&str>) -> Result<bool, String> {
    let pat = pattern.ok_or("regex_match requires 'pattern' parameter")?;
    let re = Regex::new(pat).map_err(|e| format!("Invalid regex: {}", e))?;
    for val in values {
        if let Some(s) = val.as_str() {
            if re.is_match(s) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn regex_not_match(values: &[serde_json::Value], pattern: Option<&str>) -> Result<bool, String> {
    let pat = pattern.ok_or("regex_not_match requires 'pattern' parameter")?;
    let re = Regex::new(pat).map_err(|e| format!("Invalid regex: {}", e))?;
    for val in values {
        if let Some(s) = val.as_str() {
            if !re.is_match(s) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn equals(values: &[serde_json::Value], target: &Option<serde_json::Value>) -> Result<bool, String> {
    let target = target.as_ref().ok_or("equals requires 'value' parameter")?;
    for val in values {
        if val == target {
            return Ok(true);
        }
    }
    Ok(false)
}

fn not_equals(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target = target.as_ref().ok_or("not_equals requires 'value' parameter")?;
    for val in values {
        if val != target {
            return Ok(true);
        }
    }
    Ok(false)
}

fn contains_check(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target_str = target
        .as_ref()
        .and_then(|v| v.as_str())
        .ok_or("contains requires 'value' parameter (string)")?;
    for val in values {
        if let Some(s) = val.as_str() {
            if s.contains(target_str) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn not_contains_check(
    values: &[serde_json::Value],
    target: &Option<serde_json::Value>,
) -> Result<bool, String> {
    let target_str = target
        .as_ref()
        .and_then(|v| v.as_str())
        .ok_or("not_contains requires 'value' parameter (string)")?;
    for val in values {
        if let Some(s) = val.as_str() {
            if !s.contains(target_str) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn size_exceeds(raw_bytes: usize, max: Option<usize>) -> Result<bool, String> {
    let max = max.ok_or("size_exceeds requires 'max_bytes' parameter")?;
    Ok(raw_bytes > max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params() -> ConditionParams {
        ConditionParams::default()
    }

    // --- domain_not_in ---

    #[test]
    fn test_domain_not_in_blocks_unknown_domain() {
        let vals = vec![json!("https://evil.sh/exfil")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_not_in_allows_known_domain() {
        let vals = vec![json!("https://api.safe.com/hook")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_not_in_exact_match() {
        let vals = vec![json!("https://safe.com/hook")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }

    // --- domain_in ---

    #[test]
    fn test_domain_in_blocks_blacklisted() {
        let vals = vec![json!("https://evil.sh/exfil")];
        let mut p = params();
        p.values = vec!["evil.sh".to_string()];
        assert!(evaluate_condition("domain_in", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_domain_in_allows_non_blacklisted() {
        let vals = vec![json!("https://safe.com/hook")];
        let mut p = params();
        p.values = vec!["evil.sh".to_string()];
        assert!(!evaluate_condition("domain_in", &vals, &p, 0).unwrap());
    }

    // --- regex_match ---

    #[test]
    fn test_regex_match_triggers() {
        let vals = vec![json!("8501015009087")];
        let mut p = params();
        p.pattern = Some(r"\d{13}".to_string());
        assert!(evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_regex_match_no_trigger() {
        let vals = vec![json!("hello world")];
        let mut p = params();
        p.pattern = Some(r"\d{13}".to_string());
        assert!(!evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    // --- regex_not_match ---

    #[test]
    fn test_regex_not_match_triggers() {
        let vals = vec![json!("hello")];
        let mut p = params();
        p.pattern = Some(r"^\d+$".to_string());
        assert!(evaluate_condition("regex_not_match", &vals, &p, 0).unwrap());
    }

    // --- equals ---

    #[test]
    fn test_equals_triggers() {
        let vals = vec![json!("blocked_value")];
        let mut p = params();
        p.value = Some(json!("blocked_value"));
        assert!(evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_equals_no_trigger() {
        let vals = vec![json!("ok_value")];
        let mut p = params();
        p.value = Some(json!("blocked_value"));
        assert!(!evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_equals_numeric() {
        let vals = vec![json!(42)];
        let mut p = params();
        p.value = Some(json!(42));
        assert!(evaluate_condition("equals", &vals, &p, 0).unwrap());
    }

    // --- not_equals ---

    #[test]
    fn test_not_equals_triggers() {
        let vals = vec![json!("something_else")];
        let mut p = params();
        p.value = Some(json!("expected"));
        assert!(evaluate_condition("not_equals", &vals, &p, 0).unwrap());
    }

    // --- contains ---

    #[test]
    fn test_contains_triggers() {
        let vals = vec![json!("this has evil inside")];
        let mut p = params();
        p.value = Some(json!("evil"));
        assert!(evaluate_condition("contains", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_contains_no_trigger() {
        let vals = vec![json!("this is fine")];
        let mut p = params();
        p.value = Some(json!("evil"));
        assert!(!evaluate_condition("contains", &vals, &p, 0).unwrap());
    }

    // --- not_contains ---

    #[test]
    fn test_not_contains_triggers() {
        let vals = vec![json!("missing keyword")];
        let mut p = params();
        p.value = Some(json!("required"));
        assert!(evaluate_condition("not_contains", &vals, &p, 0).unwrap());
    }

    // --- size_exceeds ---

    #[test]
    fn test_size_exceeds_triggers() {
        let mut p = params();
        p.max_bytes = Some(100);
        assert!(evaluate_condition("size_exceeds", &[], &p, 200).unwrap());
    }

    #[test]
    fn test_size_exceeds_no_trigger() {
        let mut p = params();
        p.max_bytes = Some(100);
        assert!(!evaluate_condition("size_exceeds", &[], &p, 50).unwrap());
    }

    // --- field_exists / field_not_exists ---

    #[test]
    fn test_field_exists_triggers() {
        let vals = vec![json!("anything")];
        assert!(evaluate_condition("field_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_exists_no_trigger() {
        let vals: Vec<serde_json::Value> = vec![];
        assert!(!evaluate_condition("field_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_not_exists_triggers() {
        let vals: Vec<serde_json::Value> = vec![];
        assert!(evaluate_condition("field_not_exists", &vals, &params(), 0).unwrap());
    }

    #[test]
    fn test_field_not_exists_no_trigger() {
        let vals = vec![json!("present")];
        assert!(!evaluate_condition("field_not_exists", &vals, &params(), 0).unwrap());
    }

    // --- unknown condition ---

    #[test]
    fn test_unknown_condition_errors() {
        let result = evaluate_condition("made_up", &[], &params(), 0);
        assert!(result.is_err());
    }

    // --- edge cases ---

    #[test]
    fn test_non_string_value_skipped_by_regex() {
        let vals = vec![json!(12345)];
        let mut p = params();
        p.pattern = Some(r"\d+".to_string());
        // Numeric values are not strings, so regex doesn't apply
        assert!(!evaluate_condition("regex_match", &vals, &p, 0).unwrap());
    }

    #[test]
    fn test_non_url_value_skipped_by_domain() {
        let vals = vec![json!("not a url")];
        let mut p = params();
        p.values = vec!["*.safe.com".to_string()];
        // Non-URL values can't have domains extracted, so they pass
        assert!(!evaluate_condition("domain_not_in", &vals, &p, 0).unwrap());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test conditions 2>&1 | tail -25`
Expected: All 23 tests pass.

- [ ] **Step 3: Commit**

```bash
git add guard-rail-engine/src/policy/conditions.rs
git commit -m "implement all 11 policy condition types with tests"
```

---

### Task 6: Policy Engine

**Files:**
- Modify: `guard-rail-engine/src/policy/engine.rs`

Evaluation logic: takes a payload + list of policy names, runs JSONPath extraction, evaluates conditions, returns ALLOW or BLOCK.

- [ ] **Step 1: Implement the policy engine**

Replace `guard-rail-engine/src/policy/engine.rs` with:

```rust
use super::conditions::{evaluate_condition, ConditionParams};
use super::{Policy, PolicySet};
use jsonpath_rust::JsonPath;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Verdict {
    Allow,
    Block {
        policy_name: String,
        rule_field: String,
        message: String,
        violation_value: Option<String>,
    },
}

pub fn evaluate(
    payload: &serde_json::Value,
    raw_bytes: usize,
    policy_names: &[String],
    policy_set: &PolicySet,
) -> Verdict {
    for name in policy_names {
        let policy = match policy_set.get(name) {
            Some(p) => p,
            None => {
                return Verdict::Block {
                    policy_name: name.clone(),
                    rule_field: String::new(),
                    message: format!("Policy not found: {}", name),
                    violation_value: None,
                };
            }
        };

        if let Some(block) = evaluate_policy(payload, raw_bytes, policy) {
            return block;
        }
    }

    Verdict::Allow
}

fn evaluate_policy(
    payload: &serde_json::Value,
    raw_bytes: usize,
    policy: &Policy,
) -> Option<Verdict> {
    for rule in &policy.rules {
        let extracted = extract_values(payload, &rule.field);

        let params = ConditionParams {
            values: rule.values.clone(),
            value: rule.value.clone(),
            pattern: rule.pattern.clone(),
            max_bytes: rule.max_bytes,
        };

        match evaluate_condition(&rule.condition, &extracted, &params, raw_bytes) {
            Ok(true) => {
                let violation_value = extracted.first().map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        v.to_string()
                    }
                });

                return Some(Verdict::Block {
                    policy_name: policy.name.clone(),
                    rule_field: rule.field.clone(),
                    message: format!(
                        "{} condition triggered on field {}",
                        rule.condition, rule.field
                    ),
                    violation_value,
                });
            }
            Ok(false) => continue,
            Err(e) => {
                return Some(Verdict::Block {
                    policy_name: policy.name.clone(),
                    rule_field: rule.field.clone(),
                    message: format!("Condition evaluation error: {}", e),
                    violation_value: None,
                });
            }
        }
    }

    None
}

fn extract_values(payload: &serde_json::Value, path: &str) -> Vec<serde_json::Value> {
    if path == "$" {
        return vec![payload.clone()];
    }

    let jsonpath = match JsonPath::from_str(path) {
        Ok(jp) => jp,
        Err(_) => return vec![],
    };

    let result = jsonpath.find(payload);
    match result {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Null => vec![],
        other => vec![other],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Policy, PolicySet, Rule};
    use serde_json::json;
    use std::collections::HashMap;

    fn make_policy_set(policies: Vec<Policy>) -> PolicySet {
        let mut by_name = HashMap::new();
        for p in policies {
            by_name.insert(p.name.clone(), p);
        }
        // Construct PolicySet directly for testing
        // We need to make by_name accessible — use load_dir with temp files
        // Actually, let's build it via the test helper
        PolicySet { by_name }
    }

    fn simple_domain_policy() -> Policy {
        Policy {
            name: "block-callbacks".to_string(),
            description: String::new(),
            rules: vec![Rule {
                field: "$.callback".to_string(),
                condition: "domain_not_in".to_string(),
                values: vec!["*.safe.com".to_string()],
                value: None,
                pattern: None,
                max_bytes: None,
                action: "block".to_string(),
                severity: "critical".to_string(),
            }],
        }
    }

    fn size_policy() -> Policy {
        Policy {
            name: "size-limit".to_string(),
            description: String::new(),
            rules: vec![Rule {
                field: "$".to_string(),
                condition: "size_exceeds".to_string(),
                values: vec![],
                value: None,
                pattern: None,
                max_bytes: Some(100),
                action: "block".to_string(),
                severity: "warning".to_string(),
            }],
        }
    }

    #[test]
    fn test_allow_when_all_policies_pass() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"callback": "https://api.safe.com/hook", "amount": 100});

        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_block_when_policy_triggers() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"callback": "https://evil.sh/exfil", "amount": 100});

        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        match verdict {
            Verdict::Block {
                policy_name,
                rule_field,
                ..
            } => {
                assert_eq!(policy_name, "block-callbacks");
                assert_eq!(rule_field, "$.callback");
            }
            Verdict::Allow => panic!("Expected block"),
        }
    }

    #[test]
    fn test_short_circuit_on_first_block() {
        let set = make_policy_set(vec![simple_domain_policy(), size_policy()]);
        let payload = json!({"callback": "https://evil.sh/exfil"});

        // First policy blocks, second should not be evaluated
        let verdict = evaluate(
            &payload,
            50, // under size limit
            &["block-callbacks".to_string(), "size-limit".to_string()],
            &set,
        );
        match verdict {
            Verdict::Block { policy_name, .. } => {
                assert_eq!(policy_name, "block-callbacks");
            }
            Verdict::Allow => panic!("Expected block"),
        }
    }

    #[test]
    fn test_multiple_policies_all_pass() {
        let set = make_policy_set(vec![simple_domain_policy(), size_policy()]);
        let payload = json!({"callback": "https://api.safe.com/hook"});

        let verdict = evaluate(
            &payload,
            50,
            &["block-callbacks".to_string(), "size-limit".to_string()],
            &set,
        );
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_size_exceeds_blocks() {
        let set = make_policy_set(vec![size_policy()]);
        let payload = json!({"data": "x"});

        let verdict = evaluate(&payload, 200, &["size-limit".to_string()], &set);
        assert!(matches!(verdict, Verdict::Block { .. }));
    }

    #[test]
    fn test_missing_field_passes_domain_check() {
        let set = make_policy_set(vec![simple_domain_policy()]);
        let payload = json!({"amount": 100}); // no "callback" field

        let verdict = evaluate(&payload, 50, &["block-callbacks".to_string()], &set);
        // Field not found → no values extracted → domain_not_in doesn't trigger
        assert!(matches!(verdict, Verdict::Allow));
    }

    #[test]
    fn test_empty_policy_list_allows() {
        let set = make_policy_set(vec![]);
        let payload = json!({"anything": true});

        let verdict = evaluate(&payload, 50, &[], &set);
        assert!(matches!(verdict, Verdict::Allow));
    }
}
```

- [ ] **Step 2: Make PolicySet.by_name pub(crate) for test access**

In `guard-rail-engine/src/policy/mod.rs`, change:
```rust
    by_name: HashMap<String, Policy>,
```
to:
```rust
    pub(crate) by_name: HashMap<String, Policy>,
```

- [ ] **Step 3: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test engine 2>&1 | tail -15`
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add guard-rail-engine/src/policy/engine.rs guard-rail-engine/src/policy/mod.rs
git commit -m "implement policy evaluation engine with JSONPath extraction and verdict logic"
```

---

### Task 7: Response Construction

**Files:**
- Create: `guard-rail-engine/src/proxy/response.rs`
- Create: `guard-rail-engine/src/proxy/mod.rs`

- [ ] **Step 1: Create response.rs**

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub status: String,
    pub execution_id: String,
    pub policy: String,
    pub rule_field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RejectResponse {
    pub status: String,
    pub execution_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: String,
    pub execution_id: String,
    pub message: String,
}

pub fn block_response(
    execution_id: &str,
    policy: &str,
    rule_field: &str,
    message: &str,
) -> Response {
    let body = BlockResponse {
        status: "blocked".to_string(),
        execution_id: execution_id.to_string(),
        policy: policy.to_string(),
        rule_field: rule_field.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::FORBIDDEN,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn reject_response(execution_id: &str, message: &str) -> Response {
    let body = RejectResponse {
        status: "rejected".to_string(),
        execution_id: execution_id.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::BAD_REQUEST,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn bad_gateway_response(execution_id: &str, message: &str) -> Response {
    let body = ErrorResponse {
        status: "error".to_string(),
        execution_id: execution_id.to_string(),
        message: message.to_string(),
    };
    (
        StatusCode::BAD_GATEWAY,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}

pub fn method_not_allowed_response(execution_id: &str) -> Response {
    let body = RejectResponse {
        status: "rejected".to_string(),
        execution_id: execution_id.to_string(),
        message: "Method not allowed for this route".to_string(),
    };
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [("x-guardrail-execution-id", execution_id.to_string())],
        Json(body),
    )
        .into_response()
}
```

- [ ] **Step 2: Create proxy/mod.rs placeholder**

```rust
pub mod forward;
pub mod response;
```

- [ ] **Step 3: Create proxy/forward.rs placeholder**

```rust
// Upstream forwarding — implemented in Task 9
```

- [ ] **Step 4: Add mod to main.rs**

Add to the top of `guard-rail-engine/src/main.rs`:
```rust
mod proxy;
```

- [ ] **Step 5: Verify compilation**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add guard-rail-engine/src/proxy/
git commit -m "add response construction for block, reject, bad gateway, and method not allowed"
```

---

### Task 8: Execution Logging

**Files:**
- Create: `guard-rail-engine/src/logging.rs`

- [ ] **Step 1: Create logging.rs**

```rust
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ExecutionLog {
    pub execution_id: String,
    pub timestamp: String,
    pub route_id: String,
    pub method: String,
    pub source_ip: String,
    pub verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_error: Option<String>,
    pub latency_inspect_us: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_forward_ms: Option<u128>,
    pub latency_total_ms: u128,
}

impl ExecutionLog {
    pub fn new(execution_id: String, route_id: String, method: String, source_ip: String) -> Self {
        ExecutionLog {
            execution_id,
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            route_id,
            method,
            source_ip,
            verdict: String::new(),
            policy: None,
            rule_field: None,
            violation_value: None,
            upstream: None,
            upstream_status: None,
            forward_error: None,
            latency_inspect_us: 0,
            latency_forward_ms: None,
            latency_total_ms: 0,
        }
    }

    pub fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{}", json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_log_serializes_to_json() {
        let mut log = ExecutionLog::new(
            "GR-EXE-abc123".to_string(),
            "transfer-api".to_string(),
            "POST".to_string(),
            "127.0.0.1".to_string(),
        );
        log.verdict = "BLOCKED".to_string();
        log.policy = Some("block-callbacks".to_string());
        log.rule_field = Some("$.callback".to_string());
        log.latency_inspect_us = 142;
        log.latency_total_ms = 1;

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"execution_id\":\"GR-EXE-abc123\""));
        assert!(json.contains("\"verdict\":\"BLOCKED\""));
        assert!(json.contains("\"policy\":\"block-callbacks\""));
        // Should not contain null optional fields
        assert!(!json.contains("\"upstream\""));
    }

    #[test]
    fn test_execution_log_allowed_with_forward() {
        let mut log = ExecutionLog::new(
            "GR-EXE-def456".to_string(),
            "partner".to_string(),
            "POST".to_string(),
            "10.0.0.1".to_string(),
        );
        log.verdict = "ALLOWED".to_string();
        log.upstream = Some("https://erp.internal/webhook".to_string());
        log.upstream_status = Some(200);
        log.latency_inspect_us = 80;
        log.latency_forward_ms = Some(45);
        log.latency_total_ms = 46;

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"verdict\":\"ALLOWED\""));
        assert!(json.contains("\"upstream_status\":200"));
        assert!(json.contains("\"latency_forward_ms\":45"));
    }
}
```

- [ ] **Step 2: Add mod to main.rs**

Add to the top of `guard-rail-engine/src/main.rs`:
```rust
mod logging;
```

- [ ] **Step 3: Run tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test logging 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add guard-rail-engine/src/logging.rs guard-rail-engine/src/main.rs
git commit -m "add structured execution logging with JSON output"
```

---

### Task 9: Request Forwarding

**Files:**
- Modify: `guard-rail-engine/src/proxy/forward.rs`

- [ ] **Step 1: Implement upstream forwarding**

Replace `guard-rail-engine/src/proxy/forward.rs` with:

```rust
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use reqwest::Client;
use std::time::Duration;

pub struct ForwardResult {
    pub status: u16,
    pub response: Response,
}

pub async fn forward_request(
    client: &Client,
    upstream_url: &str,
    method: &str,
    body: bytes::Bytes,
    original_headers: &HeaderMap,
    execution_id: &str,
    timeout_ms: u64,
) -> Result<ForwardResult, String> {
    let mut headers = original_headers.clone();

    // Strip Authorization header — don't leak Guard Rail API key to upstream
    headers.remove("authorization");

    // Add Guard Rail headers
    if let Ok(val) = HeaderValue::from_str(execution_id) {
        headers.insert(
            HeaderName::from_static("x-guardrail-execution-id"),
            val,
        );
    }
    if let Ok(val) = HeaderValue::from_static("ALLOW") {
        headers.insert(
            HeaderName::from_static("x-guardrail-verdict"),
            val,
        );
    }

    // Remove host header — reqwest will set it from the URL
    headers.remove("host");

    let reqwest_method = method
        .parse::<reqwest::Method>()
        .map_err(|e| format!("Invalid method: {}", e))?;

    let response = client
        .request(reqwest_method, upstream_url)
        .headers(reqwest_headers(&headers))
        .body(body)
        .timeout(Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|e| format!("Upstream request failed: {}", e))?;

    let status = response.status().as_u16();
    let resp_headers = response.headers().clone();
    let resp_body = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read upstream response: {}", e))?;

    // Build axum response from upstream response
    let mut builder = axum::http::Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));

    // Copy upstream response headers
    for (key, value) in resp_headers.iter() {
        builder = builder.header(key.as_str(), value.as_bytes());
    }

    // Add execution ID header to response
    builder = builder.header("x-guardrail-execution-id", execution_id);

    let response = builder
        .body(axum::body::Body::from(resp_body))
        .map_err(|e| format!("Failed to build response: {}", e))?
        .into_response();

    Ok(ForwardResult { status, response })
}

fn reqwest_headers(axum_headers: &HeaderMap) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in axum_headers.iter() {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_ref()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                headers.insert(name, val);
            }
        }
    }
    headers
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: Compiles. (Integration tests for forwarding come in Task 13.)

- [ ] **Step 3: Commit**

```bash
git add guard-rail-engine/src/proxy/forward.rs
git commit -m "implement upstream HTTP forwarding with header manipulation"
```

---

### Task 10: Request Handler

**Files:**
- Modify: `guard-rail-engine/src/proxy/mod.rs`

The main handler that ties together: route matching → policy evaluation → forward/block → logging.

- [ ] **Step 1: Implement the request handler**

Replace `guard-rail-engine/src/proxy/mod.rs` with:

```rust
pub mod forward;
pub mod response;

use crate::logging::ExecutionLog;
use crate::policy::engine::{evaluate, Verdict};
use crate::policy::PolicySet;
use crate::routes::RouteTable;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method};
use axum::response::Response;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub routes: Arc<RwLock<RouteTable>>,
    pub policies: Arc<RwLock<PolicySet>>,
    pub http_client: Client,
}

pub async fn handle_execute(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(route_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let total_start = Instant::now();
    let execution_id = format!("GR-EXE-{}", Uuid::new_v4());
    let source_ip = addr.ip().to_string();

    let mut log = ExecutionLog::new(
        execution_id.clone(),
        route_id.clone(),
        method.to_string(),
        source_ip,
    );

    // 1. Route lookup
    let routes = state.routes.read().await;
    let route = match routes.lookup(&route_id) {
        Some(r) => r.clone(),
        None => {
            return (axum::http::StatusCode::NOT_FOUND, "Route not found").into_response();
        }
    };
    drop(routes);

    // 2. Method check
    let method_str = method.to_string();
    if !route.methods.contains(&method_str) {
        log.verdict = "REJECTED".to_string();
        log.latency_total_ms = total_start.elapsed().as_millis();
        log.emit();
        return response::method_not_allowed_response(&execution_id);
    }

    // 3. Parse body as JSON
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            log.verdict = "REJECTED".to_string();
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            return response::reject_response(&execution_id, "Invalid JSON body");
        }
    };

    // 4. Policy evaluation
    let inspect_start = Instant::now();
    let policies = state.policies.read().await;
    let verdict = evaluate(&payload, body.len(), &route.policies, &policies);
    drop(policies);
    log.latency_inspect_us = inspect_start.elapsed().as_micros();

    match verdict {
        Verdict::Block {
            policy_name,
            rule_field,
            message,
            violation_value,
        } => {
            log.verdict = "BLOCKED".to_string();
            log.policy = Some(policy_name.clone());
            log.rule_field = Some(rule_field.clone());
            log.violation_value = violation_value;
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            return response::block_response(&execution_id, &policy_name, &rule_field, &message);
        }
        Verdict::Allow => {}
    }

    // 5. Forward to upstream
    log.upstream = Some(route.upstream.clone());
    let forward_start = Instant::now();

    match forward::forward_request(
        &state.http_client,
        &route.upstream,
        &method_str,
        body,
        &headers,
        &execution_id,
        route.timeout_ms,
    )
    .await
    {
        Ok(result) => {
            log.verdict = "ALLOWED".to_string();
            log.upstream_status = Some(result.status);
            log.latency_forward_ms = Some(forward_start.elapsed().as_millis());
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            result.response
        }
        Err(e) => {
            log.verdict = "ALLOWED".to_string();
            log.forward_error = Some(e.clone());
            log.latency_forward_ms = Some(forward_start.elapsed().as_millis());
            log.latency_total_ms = total_start.elapsed().as_millis();
            log.emit();
            response::bad_gateway_response(&execution_id, &e)
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add guard-rail-engine/src/proxy/mod.rs
git commit -m "implement request handler — route matching, policy evaluation, forwarding"
```

---

### Task 11: Main Entry Point + Server

**Files:**
- Modify: `guard-rail-engine/src/main.rs`

Complete the main.rs with CLI parsing, config loading, tracing setup, and axum server.

- [ ] **Step 1: Replace main.rs with full implementation**

```rust
mod config;
mod logging;
mod policy;
mod proxy;
mod reload;
mod routes;

use clap::Parser;
use proxy::AppState;
use reqwest::Client;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "guard-rail-engine", about = "Zero-trust execution runtime")]
struct Cli {
    /// Path to config.yaml
    #[arg(short, long, default_value = "./config/config.yaml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load config
    let app_config = config::AppConfig::load(&cli.config)?;

    // Setup tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&app_config.logging.level));

    if app_config.logging.format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .pretty()
            .init();
    }

    tracing::info!("Loading routes from {}", app_config.routes_file);
    let route_table =
        routes::RouteTable::load(&PathBuf::from(&app_config.routes_file))?;

    tracing::info!("Loading policies from {}", app_config.policies_dir);
    let policy_set =
        policy::PolicySet::load_dir(&PathBuf::from(&app_config.policies_dir))?;

    // Validate that all route policy references exist
    let required_policies = route_table.policy_names();
    policy_set
        .validate_references(&required_policies)
        .map_err(|e| format!("Policy validation failed: {}", e))?;

    let routes = Arc::new(RwLock::new(route_table));
    let policies = Arc::new(RwLock::new(policy_set));

    // Start hot-reload watcher
    let reload_routes = Arc::clone(&routes);
    let reload_policies = Arc::clone(&policies);
    reload::start_watcher(
        PathBuf::from(&app_config.routes_file),
        PathBuf::from(&app_config.policies_dir),
        reload_routes,
        reload_policies,
    )?;

    let http_client = Client::builder()
        .user_agent(&app_config.forwarding.user_agent)
        .build()?;

    let state = AppState {
        routes,
        policies,
        http_client,
    };

    let app = axum::Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(proxy::handle_execute),
        )
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            app_config.server.request_body_limit_bytes,
        ))
        .with_state(state);

    let addr: SocketAddr = format!(
        "{}:{}",
        app_config.server.host, app_config.server.port
    )
    .parse()?;

    tracing::info!("Guard Rail Engine starting on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: Compiles (reload module not yet created — create placeholder in next step).

- [ ] **Step 3: Commit**

```bash
git add guard-rail-engine/src/main.rs
git commit -m "complete main entry point with CLI, config, tracing, and server startup"
```

---

### Task 12: Hot-Reload

**Files:**
- Create: `guard-rail-engine/src/reload.rs`

File watcher for routes and policies — reloads on change, keeps old config if new config is invalid.

- [ ] **Step 1: Create reload.rs**

```rust
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

    // Watch the routes file's parent directory (to catch renames/creates)
    if let Some(parent) = routes_file.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
    }

    // Watch the policies directory
    watcher.watch(&policies_dir, RecursiveMode::Recursive)?;

    // Leak the watcher so it lives for the duration of the process
    // (In production you'd store it in the app state, but for v1 this is fine)
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
    // Reload policies first
    match PolicySet::load_dir(policies_path) {
        Ok(new_policies) => {
            // Reload routes
            match RouteTable::load(routes_path) {
                Ok(new_routes) => {
                    // Validate references before swapping
                    let required = new_routes.policy_names();
                    if let Err(e) = new_policies.validate_references(&required) {
                        tracing::warn!("Reload rejected — {}", e);
                        return;
                    }

                    *routes.write().await = new_routes;
                    *policies.write().await = new_policies;
                    tracing::info!("Routes and policies reloaded successfully");
                }
                Err(e) => {
                    tracing::warn!("Routes reload failed, keeping previous config: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Policies reload failed, keeping previous config: {}", e);
        }
    }
}
```

- [ ] **Step 2: Verify full build**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build 2>&1 | tail -5`
Expected: Full project compiles.

- [ ] **Step 3: Run all unit tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test 2>&1 | tail -20`
Expected: All unit tests pass (config, routes, policy, conditions, engine, logging).

- [ ] **Step 4: Commit**

```bash
git add guard-rail-engine/src/reload.rs
git commit -m "add hot-reload file watcher for routes and policies"
```

---

### Task 13: Integration Tests

**Files:**
- Create: `guard-rail-engine/tests/integration_test.rs`

Full request cycle tests with a mock upstream server.

- [ ] **Step 1: Create integration tests**

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower::util::ServiceExt;

// We need to import from the crate
// Since this is an integration test, we import the public API

/// Spin up a mock upstream that returns a fixed response
async fn start_mock_upstream(status: u16, body: &'static str) -> String {
    let app = axum::Router::new().route(
        "/{*path}",
        axum::routing::any(move || async move {
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                body.to_string(),
            )
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{}", addr)
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// Build the Guard Rail app with test config
async fn build_test_app(upstream_url: &str) -> Router {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path();

    // Write routes
    write_file(
        config_dir,
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: test-route
    path: /v1/execute/test-route
    upstream: {upstream_url}/api/target
    methods: [POST]
    policies: [block-callbacks, size-limit]
    timeout_ms: 5000
  - id: open-route
    path: /v1/execute/open-route
    upstream: {upstream_url}/api/open
    methods: [POST, PUT]
    policies: []
    timeout_ms: 5000
"#
        ),
    );

    // Write policies
    let policies_dir = config_dir.join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(
        &policies_dir,
        "test.yaml",
        r#"
policies:
  - name: block-callbacks
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.safe.com"]
        action: block
        severity: critical
  - name: size-limit
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 10240
        action: block
"#,
    );

    let routes_path = config_dir.join("routes.yaml");
    let route_table = guard_rail_engine::routes::RouteTable::load(&routes_path).unwrap();
    let policy_set = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();

    let state = guard_rail_engine::proxy::AppState {
        routes: Arc::new(RwLock::new(route_table)),
        policies: Arc::new(RwLock::new(policy_set)),
        http_client: Client::new(),
    };

    // Leak TempDir so files persist for the test duration
    std::mem::forget(tmp);

    Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(guard_rail_engine::proxy::handle_execute),
        )
        .with_state(state)
        .into_make_service_with_connect_info::<SocketAddr>()
        .into_router()
}

// Helper to make requests using tower::ServiceExt
async fn make_request(app: &mut Router, method: &str, path: &str, body: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .extension(SocketAddr::from(([127, 0, 0, 1], 12345)))
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = ServiceExt::<Request<Body>>::ready(&mut *app)
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap();

    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body).to_string();

    (status, text)
}

#[tokio::test]
async fn test_allow_path_forwards_to_upstream() {
    let upstream = start_mock_upstream(200, r#"{"result":"ok"}"#).await;
    let mut app = build_test_app(&upstream).await;

    let (status, body) = make_request(
        &mut app,
        "POST",
        "/v1/execute/open-route",
        r#"{"data": "hello"}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("ok"));
}

#[tokio::test]
async fn test_block_path_returns_403() {
    let upstream = start_mock_upstream(200, "ok").await;
    let mut app = build_test_app(&upstream).await;

    let (status, body) = make_request(
        &mut app,
        "POST",
        "/v1/execute/test-route",
        r#"{"callback": "https://evil.sh/exfil", "amount": 100}"#,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.contains("blocked"));
    assert!(body.contains("block-callbacks"));
}

#[tokio::test]
async fn test_allow_when_policy_passes() {
    let upstream = start_mock_upstream(200, r#"{"forwarded": true}"#).await;
    let mut app = build_test_app(&upstream).await;

    let (status, body) = make_request(
        &mut app,
        "POST",
        "/v1/execute/test-route",
        r#"{"callback": "https://api.safe.com/hook", "amount": 100}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("forwarded"));
}

#[tokio::test]
async fn test_invalid_json_returns_400() {
    let upstream = start_mock_upstream(200, "ok").await;
    let mut app = build_test_app(&upstream).await;

    let (status, body) = make_request(
        &mut app,
        "POST",
        "/v1/execute/test-route",
        "this is not json",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("rejected"));
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let upstream = start_mock_upstream(200, "ok").await;
    let mut app = build_test_app(&upstream).await;

    let (status, _) = make_request(
        &mut app,
        "POST",
        "/v1/execute/nonexistent",
        r#"{"data": 1}"#,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_wrong_method_returns_405() {
    let upstream = start_mock_upstream(200, "ok").await;
    let mut app = build_test_app(&upstream).await;

    // test-route only allows POST, not PUT
    let (status, body) = make_request(
        &mut app,
        "PUT",
        "/v1/execute/test-route",
        r#"{"data": 1}"#,
    )
    .await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(body.contains("rejected"));
}
```

- [ ] **Step 2: Make crate modules public for integration tests**

Add to the top of `guard-rail-engine/src/main.rs`, before the mod declarations:
```rust
// Public modules for integration test access
```

And change all `mod` declarations to `pub mod`:
```rust
pub mod config;
pub mod logging;
pub mod policy;
pub mod proxy;
pub mod reload;
pub mod routes;
```

Also add to `Cargo.toml` under `[lib]` section — we need both a binary and a library target:

Add after `[package]` section:
```toml
[[bin]]
name = "guard-rail-engine"
path = "src/main.rs"

[lib]
name = "guard_rail_engine"
path = "src/lib.rs"
```

Create `guard-rail-engine/src/lib.rs`:
```rust
pub mod config;
pub mod logging;
pub mod policy;
pub mod proxy;
pub mod reload;
pub mod routes;
```

Revert main.rs mod declarations back to private:
```rust
mod config;
mod logging;
mod policy;
mod proxy;
mod reload;
mod routes;
```

- [ ] **Step 3: Add bytes dev-dependency**

Add to `[dev-dependencies]` in Cargo.toml:
```toml
bytes = "1"
```

- [ ] **Step 4: Run integration tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test --test integration_test 2>&1 | tail -20`
Expected: 6 integration tests pass.

- [ ] **Step 5: Run all tests**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test 2>&1 | tail -20`
Expected: All unit + integration tests pass.

- [ ] **Step 6: Commit**

```bash
git add guard-rail-engine/src/lib.rs guard-rail-engine/src/main.rs guard-rail-engine/tests/ guard-rail-engine/Cargo.toml
git commit -m "add integration tests with mock upstream — allow, block, reject, 404, 405"
```

---

### Task 14: Example Config + Final Verification

**Files:**
- Verify: `guard-rail-engine/config/config.yaml` (already created in Task 2)
- Verify: `guard-rail-engine/config/routes.yaml` (already created in Task 3)
- Verify: `guard-rail-engine/config/policies/security.yaml` (already created in Task 4)

- [ ] **Step 1: Run the full test suite**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo test 2>&1`
Expected: All tests pass — unit tests for config, routes, policy loading, conditions, engine, logging + integration tests.

- [ ] **Step 2: Build release binary**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && cargo build --release 2>&1 | tail -5`
Expected: Release build succeeds.

- [ ] **Step 3: Verify the binary starts**

Run: `cd /home/nyasha-hama/projects/guard-rail/guard-rail-engine && timeout 3 cargo run -- --config ./config/config.yaml 2>&1 || true`
Expected: Output includes "Guard Rail Engine starting on 0.0.0.0:8080" (then times out since it's a server).

- [ ] **Step 4: Final commit**

```bash
git add -A guard-rail-engine/
git commit -m "Guard Rail Engine Stage 1 complete — core proxy + policy engine"
```

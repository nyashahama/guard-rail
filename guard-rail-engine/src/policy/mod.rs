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
    pub(crate) by_name: HashMap<String, Policy>,
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
        assert!(set.validate_references(&["exists".to_string()]).is_ok());
        assert!(set.validate_references(&["missing".to_string()]).is_err());
    }

    #[test]
    fn test_invalid_yaml_errors() {
        let dir = TempDir::new().unwrap();
        write_policy(dir.path(), "bad.yaml", "not: [valid: yaml: {{");

        let result = PolicySet::load_dir(dir.path());
        assert!(result.is_err());
    }
}
